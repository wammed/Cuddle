use crate::models::FileItem;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_tantivy::tokenizer::LinderaTokenizer;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyError};
use thiserror::Error;
use ts_rs::TS;

const APP_DIR_NAME: &str = "Cuddle";
const INDEX_SUBDIR_NAME: &str = "index";
const TOKENIZER_NAME_JA: &str = "lang_ja";
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

/// Strongly-typed Tantivy schema field references.
#[derive(Clone, Copy)]
pub struct FulltextFields {
    pub item_id: Field,
    pub raw_base64: Field,
    pub display_name: Field,
    pub body: Field,
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

/// Returns the globally shared CuddleFulltextEngine instance, lazily initialized on first access.
pub fn get_or_init_engine() -> Result<&'static CuddleFulltextEngine, FulltextError> {
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

    let string_options = STRING | STORED;

    let item_id = builder.add_text_field("item_id", string_options.clone());
    let raw_base64 = builder.add_text_field("raw_base64", string_options);
    let display_name = builder.add_text_field("display_name", text_options_ja.clone());
    let body = builder.add_text_field("body", text_options_ja);
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
        mtime_ms,
        size,
        inode,
        dev,
    };

    (schema, fields)
}

fn register_lindera_tokenizer(index: &Index) -> Result<(), FulltextError> {
    let mode = Mode::Normal;
    let dictionary =
        load_dictionary("embedded://ipadic").map_err(|e| FulltextError::Lindera(e.to_string()))?;
    let segmenter = Segmenter::new(mode, dictionary, None);
    let tokenizer = LinderaTokenizer::from_segmenter(segmenter);

    index.tokenizers().register(TOKENIZER_NAME_JA, tokenizer);
    Ok(())
}

impl CuddleFulltextEngine {
    /// Opens or creates a persistent on-disk full-text index in the given path.
    pub fn new_in_dir(path: &Path) -> Result<Self, FulltextError> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }

        let (schema, fields) = build_schema();
        let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(path)?, schema)?;
        register_lindera_tokenizer(&index)?;

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
        register_lindera_tokenizer(&index)?;

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

        // Remove previous version if already indexed
        let term = Term::from_field_text(self.fields.item_id, &item.item_id);
        writer.delete_term(term);

        let doc = doc!(
            self.fields.item_id => item.item_id.clone(),
            self.fields.raw_base64 => item.raw_base64.clone(),
            self.fields.display_name => item.display_name.clone(),
            self.fields.body => body,
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
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.fields.display_name, self.fields.body],
        );

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

        // 2. Search for "京都" (Kyoto) - Should match doc2
        let hits_kyoto = engine.search("京都", &opts).unwrap();
        assert_eq!(hits_kyoto.len(), 1);
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
    }

    #[test]
    fn test_empty_query_error() {
        let engine = CuddleFulltextEngine::new_in_ram().unwrap();
        let opts = FulltextSearchOptions::default();
        let res = engine.search("   ", &opts);
        assert!(matches!(res, Err(FulltextError::EmptyQuery)));
    }

    #[test]
    fn test_export_typescript_bindings_fulltext() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        let _ = FulltextHitDto::export(&cfg);
        let _ = FulltextSearchOptions::export(&cfg);
    }
}
