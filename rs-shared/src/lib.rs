use serde::Serialize;

#[derive(Serialize)]
pub struct SFile {
    pub is_dir: bool,
    pub full_path: String,
    pub created_at: i64,
    pub modified_at: i64,
    // Either the name of the directory or the file
    pub top_level_name: String
}