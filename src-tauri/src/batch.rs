// src-tauri/src/batch.rs
// Cuddle - Phase 5.1: Normalization & Bulk Rename Rules Engine & Dry-run Verification

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};
use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum NormalizationForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum SequencePosition {
    Prefix,
    SuffixBeforeExtension,
    SuffixAfterExtension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum DateTimeSource {
    CurrentTime,
    FileModifiedTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export, export_to = "../../src/types/")]
pub enum RenameRule {
    RegexReplace {
        pattern: String,
        replacement: String,
    },
    UnicodeNormalize {
        form: NormalizationForm,
    },
    ConvertCharacter {
        half_width_alnum: bool,
        half_width_kana: bool,
        space_to_underscore: bool,
        underscore_to_space: bool,
        to_lowercase: bool,
        to_uppercase: bool,
    },
    AddPrefixSuffix {
        prefix: String,
        suffix: String,
        preserve_extension: bool,
    },
    AddSequence {
        start: u32,
        step: u32,
        padding: usize,
        position: SequencePosition,
        preserve_extension: bool,
    },
    AddDateTime {
        format: String,
        source: DateTimeSource,
        position: SequencePosition,
        preserve_extension: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub enum OperationStatus {
    Ok,
    Unchanged,
    ConflictDuplicate,
    ConflictExistsOnDisk,
    InvalidFilename,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct PlannedOperationDto {
    pub item_id: String,
    pub original_path: String,
    pub display_name: String,
    pub raw_base64: String,
    pub proposed_name: String,
    pub target_path: String,
    pub status: OperationStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct BatchPlanDto {
    pub batch_id: String,
    pub operations: Vec<PlannedOperationDto>,
    pub total_count: usize,
    pub changed_count: usize,
    pub conflict_count: usize,
    pub error_count: usize,
    pub is_executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct RenameTargetInputDto {
    pub item_id: String,
    pub path_or_base64: String,
    pub display_name: String,
    pub mtime_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct BulkRenameOptions {
    pub target_items: Vec<RenameTargetInputDto>,
    pub rules: Vec<RenameRule>,
}

/// Helper to split filename into stem and extension properly, handling dotfiles
pub fn split_stem_and_ext(filename: &str) -> (String, Option<String>) {
    if filename.is_empty() {
        return (String::new(), None);
    }

    // Dotfiles like .gitignore, .bashrc (no extension, stem is the whole dotfile)
    if let Some(rest) = filename.strip_prefix('.') {
        if rest.find('.').is_none() {
            return (filename.to_string(), None);
        }
    }

    // Dotfiles with extra extension like .config.json
    let (is_dotfile, search_slice) = if let Some(rest) = filename.strip_prefix('.') {
        (true, rest)
    } else {
        (false, filename)
    };

    if let Some(dot_idx) = search_slice.rfind('.') {
        let actual_idx = if is_dotfile {
            dot_idx + 1
        } else {
            dot_idx
        };
        let stem = filename[..actual_idx].to_string();
        let ext = filename[actual_idx + 1..].to_string();
        (stem, Some(ext))
    } else {
        (filename.to_string(), None)
    }
}

/// Recombine stem and extension
pub fn join_stem_and_ext(stem: &str, ext: Option<&str>) -> String {
    match ext {
        Some(e) if !e.is_empty() => format!("{}.{}", stem, e),
        _ => stem.to_string(),
    }
}

/// Convert full-width ASCII characters to half-width
pub fn convert_fullwidth_alnum_to_halfwidth(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            // Full-width space (U+3000) -> standard space
            '\u{3000}' => out.push(' '),
            // Full-width ASCII: ！(U+FF01) to ～(U+FF5E) -> !(U+0021) to ~(U+007E)
            '\u{FF01}'..='\u{FF5E}' => {
                let ascii = (c as u32 - 0xFEE0) as u8 as char;
                out.push(ascii);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Apply a single rename rule to a filename
pub fn apply_rule(
    filename: &str,
    rule: &RenameRule,
    index: usize,
    modified_time: Option<DateTime<Local>>,
) -> Result<String, String> {
    match rule {
        RenameRule::RegexReplace { pattern, replacement } => {
            let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
            Ok(re.replace_all(filename, replacement.as_str()).to_string())
        }
        RenameRule::UnicodeNormalize { form } => {
            let normalized = match form {
                NormalizationForm::Nfc => filename.nfc().collect::<String>(),
                NormalizationForm::Nfd => filename.nfd().collect::<String>(),
                NormalizationForm::Nfkc => filename.nfkc().collect::<String>(),
                NormalizationForm::Nfkd => filename.nfkd().collect::<String>(),
            };
            Ok(normalized)
        }
        RenameRule::ConvertCharacter {
            half_width_alnum,
            half_width_kana: _,
            space_to_underscore,
            underscore_to_space,
            to_lowercase,
            to_uppercase,
        } => {
            let mut result = filename.to_string();
            if *half_width_alnum {
                result = convert_fullwidth_alnum_to_halfwidth(&result);
            }
            if *space_to_underscore {
                result = result.replace(' ', "_");
            }
            if *underscore_to_space {
                result = result.replace('_', " ");
            }
            if *to_lowercase {
                result = result.to_lowercase();
            } else if *to_uppercase {
                result = result.to_uppercase();
            }
            Ok(result)
        }
        RenameRule::AddPrefixSuffix {
            prefix,
            suffix,
            preserve_extension,
        } => {
            if *preserve_extension {
                let (stem, ext) = split_stem_and_ext(filename);
                let new_stem = format!("{}{}{}", prefix, stem, suffix);
                Ok(join_stem_and_ext(&new_stem, ext.as_deref()))
            } else {
                Ok(format!("{}{}{}", prefix, filename, suffix))
            }
        }
        RenameRule::AddSequence {
            start,
            step,
            padding,
            position,
            preserve_extension,
        } => {
            let seq_num = start.saturating_add((index as u32).saturating_mul(*step));
            let seq_str = format!("{:0>width$}", seq_num, width = *padding);

            if *preserve_extension {
                let (stem, ext) = split_stem_and_ext(filename);
                let new_stem = match position {
                    SequencePosition::Prefix => format!("{}{}", seq_str, stem),
                    SequencePosition::SuffixBeforeExtension | SequencePosition::SuffixAfterExtension => {
                        format!("{}{}", stem, seq_str)
                    }
                };
                Ok(join_stem_and_ext(&new_stem, ext.as_deref()))
            } else {
                match position {
                    SequencePosition::Prefix => Ok(format!("{}{}", seq_str, filename)),
                    SequencePosition::SuffixBeforeExtension | SequencePosition::SuffixAfterExtension => {
                        Ok(format!("{}{}", filename, seq_str))
                    }
                }
            }
        }
        RenameRule::AddDateTime {
            format,
            source,
            position,
            preserve_extension,
        } => {
            let dt = match source {
                DateTimeSource::CurrentTime => Local::now(),
                DateTimeSource::FileModifiedTime => modified_time.unwrap_or_else(Local::now),
            };
            let dt_str = dt.format(format).to_string();

            if *preserve_extension {
                let (stem, ext) = split_stem_and_ext(filename);
                let new_stem = match position {
                    SequencePosition::Prefix => format!("{}{}", dt_str, stem),
                    SequencePosition::SuffixBeforeExtension | SequencePosition::SuffixAfterExtension => {
                        format!("{}{}", stem, dt_str)
                    }
                };
                Ok(join_stem_and_ext(&new_stem, ext.as_deref()))
            } else {
                match position {
                    SequencePosition::Prefix => Ok(format!("{}{}", dt_str, filename)),
                    SequencePosition::SuffixBeforeExtension | SequencePosition::SuffixAfterExtension => {
                        Ok(format!("{}{}", filename, dt_str))
                    }
                }
            }
        }
    }
}

/// Apply a pipeline of rename rules to a filename sequentially
pub fn apply_rule_chain(
    initial_name: &str,
    rules: &[RenameRule],
    index: usize,
    modified_time: Option<DateTime<Local>>,
) -> Result<String, String> {
    let mut current_name = initial_name.to_string();
    for rule in rules {
        current_name = apply_rule(&current_name, rule, index, modified_time)?;
    }
    Ok(current_name)
}

/// Validate if a proposed filename is legal in Linux filesystem
pub fn validate_filename(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Filename cannot be empty");
    }
    if name.contains('\0') {
        return Err("Filename cannot contain null byte");
    }
    if name.contains('/') {
        return Err("Filename cannot contain directory separator '/'");
    }
    if name == "." || name == ".." {
        return Err("Filename cannot be '.' or '..'");
    }
    Ok(())
}

/// Item input representation for generating a batch rename plan
#[derive(Debug, Clone)]
pub struct RenameTargetItem {
    pub item_id: String,
    pub original_path: PathBuf,
    pub display_name: String,
    pub raw_base64: String,
    pub modified_time: Option<DateTime<Local>>,
}

/// Generate a BatchPlan with full dry-run collision detection
pub fn generate_batch_plan(
    items: Vec<RenameTargetItem>,
    rules: &[RenameRule],
) -> BatchPlanDto {
    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut operations = Vec::with_capacity(items.len());

    // 1. Calculate proposed names for all items
    for (idx, item) in items.iter().enumerate() {
        let parent_dir = item.original_path.parent().unwrap_or_else(|| Path::new(""));

        match apply_rule_chain(&item.display_name, rules, idx, item.modified_time) {
            Ok(proposed_name) => {
                if let Err(err_msg) = validate_filename(&proposed_name) {
                    operations.push(PlannedOperationDto {
                        item_id: item.item_id.clone(),
                        original_path: item.original_path.to_string_lossy().to_string(),
                        display_name: item.display_name.clone(),
                        raw_base64: item.raw_base64.clone(),
                        proposed_name: proposed_name.clone(),
                        target_path: parent_dir.join(&proposed_name).to_string_lossy().to_string(),
                        status: OperationStatus::InvalidFilename,
                        message: Some(err_msg.to_string()),
                    });
                } else {
                    let is_unchanged = proposed_name == item.display_name;
                    let target_path = parent_dir.join(&proposed_name);
                    let initial_status = if is_unchanged {
                        OperationStatus::Unchanged
                    } else {
                        OperationStatus::Ok
                    };

                    operations.push(PlannedOperationDto {
                        item_id: item.item_id.clone(),
                        original_path: item.original_path.to_string_lossy().to_string(),
                        display_name: item.display_name.clone(),
                        raw_base64: item.raw_base64.clone(),
                        proposed_name,
                        target_path: target_path.to_string_lossy().to_string(),
                        status: initial_status,
                        message: None,
                    });
                }
            }
            Err(e) => {
                operations.push(PlannedOperationDto {
                    item_id: item.item_id.clone(),
                    original_path: item.original_path.to_string_lossy().to_string(),
                    display_name: item.display_name.clone(),
                    raw_base64: item.raw_base64.clone(),
                    proposed_name: item.display_name.clone(),
                    target_path: item.original_path.to_string_lossy().to_string(),
                    status: OperationStatus::Error,
                    message: Some(e),
                });
            }
        }
    }

    // 2. Intra-batch collision detection (Duplicate target paths within the batch)
    let mut target_path_counts: HashMap<String, usize> = HashMap::new();
    for op in &operations {
        if op.status == OperationStatus::Ok || op.status == OperationStatus::Unchanged {
            *target_path_counts.entry(op.target_path.clone()).or_insert(0) += 1;
        }
    }

    let batch_original_paths: HashSet<String> = items
        .iter()
        .map(|it| it.original_path.to_string_lossy().to_string())
        .collect();

    // 3. Mark duplicate conflicts and check against disk collision
    for op in operations.iter_mut() {
        if op.status == OperationStatus::Ok || op.status == OperationStatus::Unchanged {
            if let Some(&count) = target_path_counts.get(&op.target_path) {
                if count > 1 {
                    op.status = OperationStatus::ConflictDuplicate;
                    op.message = Some(format!(
                        "Multiple items would be renamed to the same path: {}",
                        op.proposed_name
                    ));
                    continue;
                }
            }

            // Check if target already exists on disk and is NOT one of the batch's original items
            if op.status == OperationStatus::Ok {
                let target_p = Path::new(&op.target_path);
                if target_p.exists() && !batch_original_paths.contains(&op.target_path) {
                    op.status = OperationStatus::ConflictExistsOnDisk;
                    op.message = Some(format!(
                        "A file with name '{}' already exists on disk in destination directory",
                        op.proposed_name
                    ));
                }
            }
        }
    }

    let total_count = operations.len();
    let mut changed_count = 0;
    let mut conflict_count = 0;
    let mut error_count = 0;

    for op in &operations {
        match op.status {
            OperationStatus::Ok => changed_count += 1,
            OperationStatus::ConflictDuplicate | OperationStatus::ConflictExistsOnDisk => {
                conflict_count += 1;
            }
            OperationStatus::InvalidFilename | OperationStatus::Error => {
                error_count += 1;
            }
            OperationStatus::Unchanged => {}
        }
    }

    let is_executable = conflict_count == 0 && error_count == 0 && changed_count > 0;

    BatchPlanDto {
        batch_id,
        operations,
        total_count,
        changed_count,
        conflict_count,
        error_count,
        is_executable,
    }
}

/// Convenience entry point to generate BatchPlanDto from BulkRenameOptions
pub fn generate_plan_from_options(options: BulkRenameOptions) -> Result<BatchPlanDto, String> {
    use chrono::TimeZone;
    let mut items = Vec::with_capacity(options.target_items.len());

    for input in options.target_items {
        let resolved = crate::models::resolve_path_str_or_b64(&input.path_or_base64)
            .map_err(|e| format!("Failed to resolve path for item {}: {}", input.item_id, e))?;
        
        let path = PathBuf::from(&resolved.path_str);
        let raw_base64 = crate::models::encode_path_to_base64(&path);

        let modified_time = input.mtime_ms.and_then(|ms| {
            Local.timestamp_millis_opt(ms).single()
        });

        items.push(RenameTargetItem {
            item_id: input.item_id,
            original_path: path,
            display_name: input.display_name,
            raw_base64,
            modified_time,
        });
    }

    Ok(generate_batch_plan(items, &options.rules))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_and_join_stem_and_ext() {
        assert_eq!(split_stem_and_ext("report.pdf"), ("report".to_string(), Some("pdf".to_string())));
        assert_eq!(split_stem_and_ext(".gitignore"), (".gitignore".to_string(), None));
        assert_eq!(split_stem_and_ext(".config.json"), (".config".to_string(), Some("json".to_string())));
        assert_eq!(split_stem_and_ext("Makefile"), ("Makefile".to_string(), None));

        assert_eq!(join_stem_and_ext("report", Some("pdf")), "report.pdf");
        assert_eq!(join_stem_and_ext("Makefile", None), "Makefile");
    }

    #[test]
    fn test_unicode_normalization_nfc_nfd() {
        // Japanese Dakuten separation test (Mac style NFD -> NFC)
        // 'ガ' in NFC is '\u{30AC}', in NFD is 'カ' ('\u{30AB}') + dakuten ('\u{3099}')
        let nfd_str = "\u{30AB}\u{3099}ストロ.txt";
        let rule_nfc = RenameRule::UnicodeNormalize { form: NormalizationForm::Nfc };
        let result = apply_rule(nfd_str, &rule_nfc, 0, None).unwrap();
        assert_eq!(result, "ガストロ.txt");
    }

    #[test]
    fn test_regex_replace() {
        let rule = RenameRule::RegexReplace {
            pattern: r"^IMG_(\d+)".to_string(),
            replacement: "Photo_$1".to_string(),
        };
        let result = apply_rule("IMG_1234.JPG", &rule, 0, None).unwrap();
        assert_eq!(result, "Photo_1234.JPG");
    }

    #[test]
    fn test_character_conversion_fullwidth_and_space() {
        let rule = RenameRule::ConvertCharacter {
            half_width_alnum: true,
            half_width_kana: false,
            space_to_underscore: true,
            underscore_to_space: false,
            to_lowercase: true,
            to_uppercase: false,
        };
        // 'ＡＢＣ １２３.txt' -> 'abc_123.txt'
        let result = apply_rule("ＡＢＣ １２３.txt", &rule, 0, None).unwrap();
        assert_eq!(result, "abc_123.txt");
    }

    #[test]
    fn test_sequence_number_with_padding() {
        let rule = RenameRule::AddSequence {
            start: 1,
            step: 1,
            padding: 3,
            position: SequencePosition::SuffixBeforeExtension,
            preserve_extension: true,
        };
        let result0 = apply_rule("document.pdf", &rule, 0, None).unwrap();
        let result1 = apply_rule("document.pdf", &rule, 1, None).unwrap();
        assert_eq!(result0, "document001.pdf");
        assert_eq!(result1, "document002.pdf");
    }

    #[test]
    fn test_dry_run_duplicate_conflict_detection() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_batch_dup_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let p1 = temp_dir.join("a.txt");
        let p2 = temp_dir.join("b.txt");

        let items = vec![
            RenameTargetItem {
                item_id: "1".to_string(),
                original_path: p1,
                display_name: "a.txt".to_string(),
                raw_base64: "YS50eHQ=".to_string(),
                modified_time: None,
            },
            RenameTargetItem {
                item_id: "2".to_string(),
                original_path: p2,
                display_name: "b.txt".to_string(),
                raw_base64: "Yi50eHQ=".to_string(),
                modified_time: None,
            },
        ];

        // Rule: change all names to fixed "fixed.txt"
        let rule = RenameRule::RegexReplace {
            pattern: ".*".to_string(),
            replacement: "fixed.txt".to_string(),
        };

        let plan = generate_batch_plan(items, &[rule]);
        assert_eq!(plan.conflict_count, 2);
        assert!(!plan.is_executable);
        assert_eq!(plan.operations[0].status, OperationStatus::ConflictDuplicate);
        assert_eq!(plan.operations[1].status, OperationStatus::ConflictDuplicate);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dry_run_disk_conflict_detection() {
        let temp_dir = std::env::temp_dir().join(format!("cuddle_batch_disk_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let p1 = temp_dir.join("source.txt");
        let target_on_disk = temp_dir.join("existing.txt");
        std::fs::write(&target_on_disk, "already exists").unwrap();

        let items = vec![RenameTargetItem {
            item_id: "1".to_string(),
            original_path: p1,
            display_name: "source.txt".to_string(),
            raw_base64: "c291cmNlLnR4dA==".to_string(),
            modified_time: None,
        }];

        let rule = RenameRule::RegexReplace {
            pattern: "source".to_string(),
            replacement: "existing".to_string(),
        };

        let plan = generate_batch_plan(items, &[rule]);
        assert_eq!(plan.conflict_count, 1);
        assert!(!plan.is_executable);
        assert_eq!(plan.operations[0].status, OperationStatus::ConflictExistsOnDisk);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_export_typescript_bindings_batch() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        BatchPlanDto::export(&cfg).expect("Failed to export BatchPlanDto");
        RenameRule::export(&cfg).expect("Failed to export RenameRule");
        NormalizationForm::export(&cfg).expect("Failed to export NormalizationForm");
        SequencePosition::export(&cfg).expect("Failed to export SequencePosition");
        DateTimeSource::export(&cfg).expect("Failed to export DateTimeSource");
        OperationStatus::export(&cfg).expect("Failed to export OperationStatus");
        PlannedOperationDto::export(&cfg).expect("Failed to export PlannedOperationDto");
        RenameTargetInputDto::export(&cfg).expect("Failed to export RenameTargetInputDto");
        BulkRenameOptions::export(&cfg).expect("Failed to export BulkRenameOptions");
    }
}
