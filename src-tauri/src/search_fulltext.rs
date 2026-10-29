use crate::models::FileItem;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_tantivy::tokenizer::LinderaTokenizer;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tantivy::collector::TopDocs;
use tantivy::query::{QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyError};
use thiserror::Error;
use ts_rs::TS;

const APP_DIR_NAME: &str = "Cuddle";
const INDEX_SUBDIR_NAME: &str = "index";
const TOKENIZER_NAME_JA: &str = "lang_ja";
const TOKENIZER_NAME_NGRAM: &str = "lang_ngram";
const WRITER_MEMORY_BUDGET_BYTES: usize = 50 * 1024 * 1024; // 50 MiB bounded memory

#[derive(Error, Debug)]
pub enum FulltextError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] TantivyError),
    #[error("Tantivy directory error: {0}")]
    Directory(#[from] tantivy::directory::error::OpenDirectoryError),
    #[error("Tantivy query parser error: {0}")]
    QueryParser(#[from] tantivy::query::QueryParserError),
    #[error("Lindera tokenizer error: {0}")]
    Lindera(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("XDG data directory could not be resolved")]
    PathResolutionFailed,
    #[error("Empty search query")]
    EmptyQuery,
}

/// A search hit result containing snippet and relevance score.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct FulltextHitDto {
    pub item_id: String,
    pub raw_base64: String,
    pub display_name: String,
    pub snippet: String,
    pub score: f32,
    pub size: u64,
    pub mtime_ms: i64,
}

/// Options controlling full-text queries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct FulltextSearchOptions {
    /// Maximum number of search results (default: 20)
    pub limit: usize,
    /// Maximum character length for preview snippets (default: 150)
    pub snippet_max_num_chars: usize,
}

impl Default for FulltextSearchOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            snippet_max_num_chars: 150,
        }
    }
}

/// Statistics on the full-text search index state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct IndexStatsDto {
    pub total_documents: usize,
    pub index_path: String,
}

/// Strongly-typed Tantivy schema field references.
#[derive(Clone, Copy)]
pub struct FulltextFields {
    pub item_id: Field,
    pub raw_base64: Field,
    pub display_name: Field,
    pub body: Field,
    pub body_ngram: Field,
    pub mtime_ms: Field,
    pub size: Field,
    pub inode: Field,
    pub dev: Field,
}

/// Full-text search engine powered by Tantivy and Lindera Japanese morphological analysis.
#[derive(Clone)]
pub struct CuddleFulltextEngine {
    index: Index,
    fields: FulltextFields,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
}

static GLOBAL_ENGINE: std::sync::OnceLock<CuddleFulltextEngine> = std::sync::OnceLock::new();
static INIT_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Returns the globally shared CuddleFulltextEngine instance, lazily initialized on first access.
pub fn get_or_init_engine() -> Result<&'static CuddleFulltextEngine, FulltextError> {
    if let Some(engine) = GLOBAL_ENGINE.get() {
        return Ok(engine);
    }

    let _guard = INIT_MUTEX.lock().unwrap();
    if let Some(engine) = GLOBAL_ENGINE.get() {
        return Ok(engine);
    }

    let index_dir = get_default_index_dir()?;
    let engine = CuddleFulltextEngine::new_in_dir(&index_dir)?;
    let _ = GLOBAL_ENGINE.set(engine);
    Ok(GLOBAL_ENGINE.get().unwrap())
}

/// Returns the XDG standard persistent full-text index storage directory:
/// `$XDG_DATA_HOME/Cuddle/index/` (or `~/.local/share/Cuddle/index/`).
pub fn get_default_index_dir() -> Result<PathBuf, FulltextError> {
    let base = dirs::data_dir().ok_or(FulltextError::PathResolutionFailed)?;
    Ok(base.join(APP_DIR_NAME).join(INDEX_SUBDIR_NAME))
}

fn build_schema() -> (Schema, FulltextFields) {
    let mut builder = Schema::builder();

    let text_indexing_ja = TextFieldIndexing::default()
        .set_tokenizer(TOKENIZER_NAME_JA)
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);

    let text_options_ja = TextOptions::default()
        .set_indexing_options(text_indexing_ja)
        .set_stored();

    // N-gram tokenizer for symbols, codes, and unknown terms (Task 1-2)
    let text_indexing_ngram = TextFieldIndexing::default()
        .set_tokenizer(TOKENIZER_NAME_NGRAM)
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);

    let text_options_ngram = TextOptions::default().set_indexing_options(text_indexing_ngram);

    let string_options = STRING | STORED;

    let item_id = builder.add_text_field("item_id", string_options.clone());
    let raw_base64 = builder.add_text_field("raw_base64", string_options);
    let display_name = builder.add_text_field("display_name", text_options_ja.clone());
    let body = builder.add_text_field("body", text_options_ja);
    let body_ngram = builder.add_text_field("body_ngram", text_options_ngram);
    let mtime_ms = builder.add_i64_field("mtime_ms", INDEXED | STORED | FAST);
    let size = builder.add_u64_field("size", INDEXED | STORED | FAST);
    let inode = builder.add_u64_field("inode", STORED | FAST);
    let dev = builder.add_u64_field("dev", STORED | FAST);

    let schema = builder.build();
    let fields = FulltextFields {
        item_id,
        raw_base64,
        display_name,
        body,
        body_ngram,
        mtime_ms,
        size,
        inode,
        dev,
    };

    (schema, fields)
}

fn register_tokenizers(index: &Index) -> Result<(), FulltextError> {
    // 1. Japanese morphological analyzer (IPADIC)
    let mode = Mode::Normal;
    let dictionary =
        load_dictionary("embedded://ipadic").map_err(|e| FulltextError::Lindera(e.to_string()))?;
    let segmenter = Segmenter::new(mode, dictionary, None);
    let tokenizer = LinderaTokenizer::from_segmenter(segmenter);
    index.tokenizers().register(TOKENIZER_NAME_JA, tokenizer);

    // 2. 2-to-3 gram tokenizer for unknown words, symbols, and code identifiers (Task 1-2)
    let ngram_tokenizer = tantivy::tokenizer::NgramTokenizer::all_ngrams(2, 3)
        .map_err(|e| FulltextError::Lindera(format!("NgramTokenizer error: {}", e)))?;
    index
        .tokenizers()
        .register(TOKENIZER_NAME_NGRAM, ngram_tokenizer);

    Ok(())
}

impl CuddleFulltextEngine {
    fn clean_index_dir(path: &Path) {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(&p);
                } else {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
    }

    /// Opens or creates a persistent on-disk full-text index in the given path.
    /// If an existing index schema does not match (e.g. across version upgrades),
    /// the stale index is automatically re-initialized with the current schema.
    pub fn new_in_dir(path: &Path) -> Result<Self, FulltextError> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }

        let (schema, fields) = build_schema();
        let mmap_dir = tantivy::directory::MmapDirectory::open(path)?;
        let index = match Index::open_or_create(mmap_dir, schema.clone()) {
            Ok(idx) => idx,
            Err(TantivyError::SchemaError(err_msg)) => {
                eprintln!(
                    "[Fulltext] Tantivy schema mismatch ({}), auto-recreating index at {:?}",
                    err_msg, path
                );
                Self::clean_index_dir(path);
                Index::create_in_dir(path, schema)?
            }
            Err(TantivyError::IncompatibleIndex(incompat)) => {
                eprintln!(
                    "[Fulltext] Tantivy incompatible index ({:?}), auto-recreating index at {:?}",
                    incompat, path
                );
                Self::clean_index_dir(path);
                Index::create_in_dir(path, schema)?
            }
            Err(e) => return Err(FulltextError::Tantivy(e)),
        };

        register_tokenizers(&index)?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        let writer = Arc::new(Mutex::new(index.writer(WRITER_MEMORY_BUDGET_BYTES)?));

        Ok(Self {
            index,
            fields,
            reader,
            writer,
        })
    }

    /// Creates an in-memory index, ideal for testing and ephemeral sessions.
    pub fn new_in_ram() -> Result<Self, FulltextError> {
        let (schema, fields) = build_schema();
        let index = Index::create_in_ram(schema);
        register_tokenizers(&index)?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        let writer = Arc::new(Mutex::new(index.writer(WRITER_MEMORY_BUDGET_BYTES)?));

        Ok(Self {
            index,
            fields,
            reader,
            writer,
        })
    }

    /// Indexes or incrementally updates a file's full-text content.
    /// Safely deletes any existing document with the same `item_id` first.
    pub fn index_document(&self, item: &FileItem, body: &str) -> Result<(), FulltextError> {
        let writer = self.writer.lock().unwrap();

        // Remove previous version if already indexed by item_id or raw_base64
        let term_id = Term::from_field_text(self.fields.item_id, &item.item_id);
        writer.delete_term(term_id);
        let term_path = Term::from_field_text(self.fields.raw_base64, &item.raw_base64);
        writer.delete_term(term_path);

        let doc = doc!(
            self.fields.item_id => item.item_id.clone(),
            self.fields.raw_base64 => item.raw_base64.clone(),
            self.fields.display_name => item.display_name.clone(),
            self.fields.body => body,
            self.fields.body_ngram => body,
            self.fields.mtime_ms => item.mtime_ms,
            self.fields.size => item.size,
            self.fields.inode => item.inode,
            self.fields.dev => item.dev,
        );

        writer.add_document(doc)?;
        Ok(())
    }

    /// Deletes a document by item_id from the index.
    pub fn delete_document(&self, item_id: &str) -> Result<(), FulltextError> {
        let writer = self.writer.lock().unwrap();
        let term = Term::from_field_text(self.fields.item_id, item_id);
        writer.delete_term(term);
        Ok(())
    }

    /// Deletes a document by its Base64-encoded path from the index (Repair Queue support).
    pub fn delete_document_by_raw_base64(&self, raw_base64: &str) -> Result<(), FulltextError> {
        let writer = self.writer.lock().unwrap();
        let term = Term::from_field_text(self.fields.raw_base64, raw_base64);
        writer.delete_term(term);
        Ok(())
    }

    /// Deletes all documents from the full-text index.
    pub fn clear_all_documents(&self) -> Result<(), FulltextError> {
        let mut writer = self.writer.lock().unwrap();
        writer.delete_all_documents()?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Returns the total number of currently indexed active documents.
    pub fn get_total_documents(&self) -> usize {
        self.reader.searcher().num_docs() as usize
    }

    /// Looks up the indexed `mtime_ms` for a file by its Base64 path, if indexed.
    pub fn get_indexed_mtime(&self, raw_base64: &str) -> Option<i64> {
        let searcher = self.reader.searcher();
        let term = Term::from_field_text(self.fields.raw_base64, raw_base64);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        if let Ok(docs) = searcher.search(&query, &TopDocs::with_limit(1).order_by_score()) {
            if let Some((_score, doc_addr)) = docs.first() {
                if let Ok(doc) = searcher.doc::<TantivyDocument>(*doc_addr) {
                    return doc.get_first(self.fields.mtime_ms).and_then(|v| v.as_i64());
                }
            }
        }
        None
    }

    /// Commits all pending indexing/deletion operations and reloads the searcher.
    pub fn commit(&self) -> Result<(), FulltextError> {
        let mut writer = self.writer.lock().unwrap();
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Searches the index across both `display_name` and `body` fields,
    /// generating highlighted preview snippets for matching body content.
    pub fn search(
        &self,
        query_str: &str,
        options: &FulltextSearchOptions,
    ) -> Result<Vec<FulltextHitDto>, FulltextError> {
        let trimmed = query_str.trim();
        if trimmed.is_empty() {
            return Err(FulltextError::EmptyQuery);
        }

        let searcher = self.reader.searcher();
        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.display_name,
                self.fields.body,
                self.fields.body_ngram,
            ],
        );
        // Boost morphological matches and display_name, while allowing n-gram hits (Task 1-2)
        query_parser.set_field_boost(self.fields.display_name, 3.0);
        query_parser.set_field_boost(self.fields.body, 2.0);
        query_parser.set_field_boost(self.fields.body_ngram, 0.8);

        let query = query_parser.parse_query(trimmed)?;
        let top_docs =
            searcher.search(&query, &TopDocs::with_limit(options.limit).order_by_score())?;

        let snippet_generator =
            tantivy::snippet::SnippetGenerator::create(&searcher, &*query, self.fields.body)?;

        let mut hits = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let item_id = retrieved_doc
                .get_first(self.fields.item_id)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let raw_base64 = retrieved_doc
                .get_first(self.fields.raw_base64)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let display_name = retrieved_doc
                .get_first(self.fields.display_name)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let size = retrieved_doc
                .get_first(self.fields.size)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let mtime_ms = retrieved_doc
                .get_first(self.fields.mtime_ms)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let snippet = snippet_generator.snippet_from_doc(&retrieved_doc).to_html();

            hits.push(FulltextHitDto {
                item_id,
                raw_base64,
                display_name,
                snippet,
                score,
                size,
                mtime_ms,
            });
        }

        Ok(hits)
    }
}

/// Safely registers an indexing failure to the persistent repair queue (Task 1-1).
pub fn enqueue_repair_on_failure(path_base64: &str, action: &str, error: &str) {
    let _ = crate::history::get_history_manager().enqueue_index_repair(path_base64, action, error);
}

/// Processes pending failed or queued index operations from the SQLite repair queue (Task 1-1).
pub fn process_index_repair_queue() -> Result<crate::history::IndexRepairResultDto, FulltextError> {
    let history_mgr = crate::history::get_history_manager();
    let pending_items = history_mgr
        .list_pending_repairs(50)
        .map_err(|e| FulltextError::Lindera(format!("Failed to list pending repairs: {}", e)))?;

    let engine = get_or_init_engine()?;
    let mut succeeded_count = 0;
    let mut failed_count = 0;

    for item in &pending_items {
        let result = match item.action.as_str() {
            "index" => match crate::extractor::index_file_with_text(&item.path_base64, None) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Index extraction error: {}", e)),
            },
            "delete" => match engine.delete_document_by_raw_base64(&item.path_base64) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Delete error: {}", e)),
            },
            other => Err(format!("Unknown repair action: {}", other)),
        };

        match result {
            Ok(()) => {
                let _ = history_mgr.delete_repair_item(item.id);
                succeeded_count += 1;
            }
            Err(err) => {
                let _ = history_mgr.increment_repair_retry(item.id, &err);
                failed_count += 1;
            }
        }
    }

    if succeeded_count > 0 {
        engine.commit()?;
    }

    let remaining_count = history_mgr.get_index_repair_queue_count().unwrap_or(0);

    Ok(crate::history::IndexRepairResultDto {
        processed_count: pending_items.len(),
        succeeded_count,
        failed_count,
        remaining_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_item(id: &str, name: &str) -> FileItem {
        FileItem {
            item_id: id.to_string(),
            raw_base64: format!("L3Rlc3QvcGF0aC/{}", name),
            display_name: name.to_string(),
            parent_raw_base64: None,
            is_dir: false,
            is_symlink: false,
            size: 1024,
            mtime_ms: 1729750000000,
            readonly: false,
            inode: 12345,
            dev: 66305,
        }
    }

    #[test]
    fn test_japanese_fulltext_search_and_snippet() {
        let engine = CuddleFulltextEngine::new_in_ram().unwrap();

        let doc1 = make_test_item("doc-1", "東京案内.md");
        let body1 =
            "羽田空港は東京都大田区にある日本最大の空港です。国際線ターミナルも充実しています。";
        engine.index_document(&doc1, body1).unwrap();

        let doc2 = make_test_item("doc-2", "関西旅行記.txt");
        let body2 =
            "京都の寺院巡りと大阪の美味しいグルメを満喫しました。新幹線での移動も快適でした。";
        engine.index_document(&doc2, body2).unwrap();

        engine.commit().unwrap();

        // 1. Search for "空港" (airport) - Should match doc1
        let opts = FulltextSearchOptions::default();
        let hits = engine.search("空港", &opts).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].item_id, "doc-1");
        assert!(hits[0].snippet.contains("空港"));

        // 2. Search for "京都" (Kyoto) - doc2 matches morpheme with higher score, doc1 matches n-gram inside "東京都"
        let hits_kyoto = engine.search("京都", &opts).unwrap();
        assert!(!hits_kyoto.is_empty());
        assert_eq!(hits_kyoto[0].item_id, "doc-2");
        assert!(hits_kyoto[0].snippet.contains("京都"));

        // 3. Search for compound word "国際線"
        let hits_intl = engine.search("国際線", &opts).unwrap();
        assert_eq!(hits_intl.len(), 1);
        assert_eq!(hits_intl[0].item_id, "doc-1");

        // 4. Update doc1 content and verify incremental replacement
        let body1_updated = "成田空港と羽田空港の二大空港の比較記事です。";
        engine.index_document(&doc1, body1_updated).unwrap();
        engine.commit().unwrap();

        let hits_after = engine.search("空港", &opts).unwrap();
        assert_eq!(hits_after.len(), 1); // Document must not be duplicated
        assert!(hits_after[0].snippet.contains("成田"));

        // 5. Delete document
        engine.delete_document("doc-1").unwrap();
        engine.commit().unwrap();
        let hits_deleted = engine.search("成田", &opts).unwrap();
        assert!(hits_deleted.is_empty());

        // 6. Test clear all and stats
        assert_eq!(engine.get_total_documents(), 1); // doc-2 remains
        assert!(engine.get_indexed_mtime(&doc2.raw_base64).is_some());
        engine.clear_all_documents().unwrap();
        assert_eq!(engine.get_total_documents(), 0);
        assert!(engine.get_indexed_mtime(&doc2.raw_base64).is_none());
    }

    #[test]
    fn test_english_code_search() {
        let engine = CuddleFulltextEngine::new_in_ram().unwrap();
        let doc = make_test_item("doc-code", "server.rs");
        let body = "pub async fn handle_request(req: Request) -> Result<Response, Error> { let token = \"auth_secret\"; }";
        engine.index_document(&doc, body).unwrap();
        engine.commit().unwrap();

        let opts = FulltextSearchOptions::default();
        let hits_token = engine.search("auth_secret", &opts).unwrap();
        assert_eq!(hits_token.len(), 1);

        let hits_req = engine.search("handle_request", &opts).unwrap();
        assert_eq!(hits_req.len(), 1);

        let hits_case = engine.search("Request", &opts).unwrap();
        assert_eq!(hits_case.len(), 1);

        let hits_lower = engine.search("request", &opts).unwrap();
        assert_eq!(hits_lower.len(), 1);
    }

    #[test]
    fn test_empty_query_error() {
        let engine = CuddleFulltextEngine::new_in_ram().unwrap();
        let opts = FulltextSearchOptions::default();
        let res = engine.search("   ", &opts);
        assert!(matches!(res, Err(FulltextError::EmptyQuery)));
    }

    #[test]
    fn test_ngram_and_morpheme_hybrid_search() {
        let engine = CuddleFulltextEngine::new_in_ram().unwrap();
        let doc = make_test_item("doc-ngram", "config.rs");
        // Body contains unknown word / code identifiers with underscores and mixed terms
        let body = "cuddle_landlock_guardrail_v5 と セキュア暗号モジュールXYZ99";
        engine.index_document(&doc, body).unwrap();
        engine.commit().unwrap();

        let opts = FulltextSearchOptions::default();

        // 1. Search for substring code identifier with underscore (hits n-gram index)
        let hits_code = engine.search("landlock_guardrail", &opts).unwrap();
        assert_eq!(hits_code.len(), 1);
        assert_eq!(hits_code[0].item_id, "doc-ngram");

        // 2. Search for mixed Katakana + Alphanumeric (XYZ99)
        let hits_mixed = engine.search("XYZ99", &opts).unwrap();
        assert_eq!(hits_mixed.len(), 1);

        // 3. Search for Japanese morpheme word
        let hits_ja = engine.search("セキュア", &opts).unwrap();
        assert_eq!(hits_ja.len(), 1);
    }

    #[test]
    fn test_index_repair_queue_workflow() {
        let history_mgr = crate::history::get_history_manager();

        // Enqueue a failed indexing task
        let path_b64 = "L2hvbWUvc3VzaWUvZG9jLnR4dA==";
        history_mgr
            .enqueue_index_repair(path_b64, "index", "Simulated Tantivy Lock Error")
            .unwrap();

        let count = history_mgr.get_index_repair_queue_count().unwrap();
        assert!(count >= 1);

        let items = history_mgr.list_pending_repairs(10).unwrap();
        let target = items.iter().find(|i| i.path_base64 == path_b64).unwrap();
        assert_eq!(target.action, "index");
        assert_eq!(target.retry_count, 0);

        // Increment retry
        history_mgr
            .increment_repair_retry(target.id, "Second retry failure")
            .unwrap();
        let items_after = history_mgr.list_pending_repairs(10).unwrap();
        let target_after = items_after.iter().find(|i| i.id == target.id).unwrap();
        assert_eq!(target_after.retry_count, 1);

        // Delete processed item
        history_mgr.delete_repair_item(target.id).unwrap();
        let items_final = history_mgr.list_pending_repairs(10).unwrap();
        assert!(!items_final.iter().any(|i| i.id == target.id));
    }

    #[test]
    fn test_schema_mismatch_auto_recovery() {
        let temp_dir =
            std::env::temp_dir().join(format!("cuddle_test_schema_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Manually create an index with an obsolete/incompatible schema
        let mut old_builder = Schema::builder();
        old_builder.add_text_field("legacy_field", TEXT);
        let old_schema = old_builder.build();

        let _old_idx = Index::create_in_dir(&temp_dir, old_schema).unwrap();

        // 2. Open via CuddleFulltextEngine::new_in_dir
        // Prior to auto-migration, this threw TantivyError::SchemaError: 'An index exists but the schema does not match.'
        let engine = CuddleFulltextEngine::new_in_dir(&temp_dir)
            .expect("Should auto-recover from schema mismatch");

        // 3. Verify the engine works and can index documents with the new schema
        let doc = make_test_item("recovered-doc", "test.txt");
        engine
            .index_document(&doc, "Auto recovery succeeded!")
            .unwrap();
        engine.commit().unwrap();

        let hits = engine
            .search("Auto", &FulltextSearchOptions::default())
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].item_id, "recovered-doc");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_fulltext() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        let _ = FulltextHitDto::export(&cfg);
        let _ = FulltextSearchOptions::export(&cfg);
        let _ = IndexStatsDto::export(&cfg);
    }
}
