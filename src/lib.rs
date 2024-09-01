use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Clone)]
struct Metadata {
    created_at: SystemTime,
    modified_at: SystemTime,
    accessed_at: SystemTime,
    size: usize,
    permissions: Permissions,
    owner: String,
    group: String,
    is_read_only: bool,
    is_hidden: bool,
    mime_type: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct Permissions {
    read: bool,
    write: bool,
    execute: bool,
}

#[derive(Debug, Clone)]
struct File {
    name: String,
    content: Vec<u8>,
    metadata: Metadata,
}

#[derive(Debug, Clone)]
struct Directory {
    name: String,
    nodes: HashMap<String, FSNode>,
    metadata: Metadata,
}

#[derive(Debug, Clone)]
enum FSNode {
    File(File),
    Directory(Directory),
}

pub struct FileSystem {
    root: Directory,
}

impl FileSystem {
    pub fn new() -> FileSystem {
        let root_metadata = Metadata {
            created_at: SystemTime::now(),
            modified_at: SystemTime::now(),
            accessed_at: SystemTime::now(),
            size: 0,
            permissions: Permissions {
                read: true,
                write: true,
                execute: false,
            },
            owner: "root".to_string(),
            group: "root".to_string(),
            is_read_only: false,
            is_hidden: false,
            mime_type: "directory".to_string(),
            tags: vec![],
        };

        FileSystem {
            root: Directory {
                name: "/".to_string(),
                nodes: HashMap::new(),
                metadata: root_metadata,
            },
        }
    }
}

impl Metadata {
    fn update_accessed(&mut self) {
        self.accessed_at = SystemTime::now();
    }

    fn update_modified(&mut self) {
        self.modified_at = SystemTime::now();
    }
}
impl FileSystem {
    pub fn create(
        &mut self,
        path: &str,
        content: Option<Vec<u8>>,
        is_directory: bool,
    ) -> Result<(), String> {
        let mut parts = path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>();
        if parts.is_empty() {
            return Err("Invalid path provided.".to_string());
        }

        let name = parts.pop().unwrap().to_string();
        let parent_dir = self.navigate_to_directory(&parts)?;

        if parent_dir.nodes.contains_key(&name.clone()) {
            return Err("File or directory already exists.".to_string());
        }

        let metadata = Metadata::default();

        if is_directory {
            let name_clone = name.clone();
            parent_dir.nodes.insert(
                name,
                FSNode::Directory(Directory {
                    name: name_clone.clone(),
                    nodes: HashMap::new(),
                    metadata,
                }),
            );
        } else {
            let name_clone = name.clone();
            parent_dir.nodes.insert(
                name,
                FSNode::File(File {
                    name: name_clone.clone(),
                    content: content.unwrap_or_default(),
                    metadata,
                }),
            );
        }

        Ok(())
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, String> {
        let (dir, filename) = self.find_node(path)?;
        match dir.nodes.get(filename) {
            Some(FSNode::File(file)) => {
                let mut metadata = file.metadata.clone();
                metadata.update_accessed();
                Ok(file.content.clone())
            }
            Some(FSNode::Directory(_)) => Err("Path points to a directory.".to_string()),
            None => Err("File not found.".to_string()),
        }
    }

    pub fn write_file(&mut self, path: &str, content: Vec<u8>, append: bool) -> Result<(), String> {
        let (dir, filename) = self.find_node(path)?;
        let mut dir_node = dir.nodes.clone();
        match dir_node.get_mut(filename) {
            Some(FSNode::File(file)) => {
                if append {
                    file.content.extend(content);
                } else {
                    file.content = content;
                }
                file.metadata.update_modified();
                Ok(())
            }
            Some(FSNode::Directory(_)) => Err("Path points to a directory.".to_string()),
            None => Err("File not found.".to_string()),
        }
    }

    pub fn list_directory(&self, path: &str) -> Result<Vec<String>, String> {
        let (dir, _) = self.find_node(path)?;
        Ok(dir.nodes.keys().cloned().collect())
    }

    fn navigate_to_directory(&mut self, parts: &[&str]) -> Result<&mut Directory, String> {
        let mut current = &mut self.root;
        for part in parts {
            match current.nodes.get_mut(*part) {
                Some(FSNode::Directory(dir)) => current = dir,
                _ => return Err("Directory not found.".to_string()),
            }
        }
        Ok(current)
    }

    fn find_node(&self, path: &str) -> Result<(&Directory, &str), String> {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let filename = parts.last().ok_or_else(|| "Invalid path.".to_string())?;
        let dir = *self.navigate_to_directory(&parts[..parts.len() - 1])?;
        Ok((&dir, filename))
    }

    pub fn delete(&mut self, path: &str) -> Result<(), String> {
        let mut parts = path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>();
        if parts.len() < 1 {
            return Err("Invalid path provided.".to_string());
        }
        let name = parts.pop().unwrap();
        let parent_dir = self.navigate_to_directory(&parts)?;

        if let Some(node) = parent_dir.nodes.remove(name) {
            match node {
                FSNode::Directory(dir) => {
                    if !dir.nodes.is_empty() {
                        return Err("Directory is not empty.".to_string());
                    }
                }
                _ => {}
            }
            Ok(())
        } else {
            Err("File or directory not found.".to_string())
        }
    }

    pub fn update_file(
        &mut self,
        path: &str,
        content: Vec<u8>,
        append: bool,
    ) -> Result<(), String> {
        let (dir, filename) = self.find_node(path)?;
        let mut dir_nodes = dir.nodes.clone();
        if let Some(FSNode::File(file)) = dir_nodes.get_mut(filename) {
            if !file.metadata.permissions.write {
                return Err("Write permission denied.".to_string());
            }

            if append {
                file.content.extend(content);
            } else {
                file.content = content;
            }
            file.metadata.update_modified();
            Ok(())
        } else {
            Err("File not found.".to_string())
        }
    }

    pub fn change_permissions(
        &mut self,
        path: &str,
        permissions: Permissions,
    ) -> Result<(), String> {
        let (dir, filename) = self.find_node(path)?;
        if let Some(node) = dir.nodes.clone().get_mut(filename) {
            node.metadata().permissions = permissions;
            node.metadata().update_modified();
            Ok(())
        } else {
            Err("File or directory not found.".to_string())
        }
    }

    pub fn search_by_tag(&self, tag: &str) -> Result<Vec<String>, String> {
        let mut results = Vec::new();
        self.search_by_tag_recursive(&self.root, tag, &mut results);
        Ok(results)
    }

    
    fn search_by_tag_recursive(&self, dir: &Directory, tag: &str, results: &mut Vec<String>) {
        for (name, node) in &dir.nodes {
            match node {
                FSNode::File(file) if file.metadata.tags.contains(&tag.to_string()) => {
                    results.push(format!("{}/{}", dir.name, file.name));
                }
                FSNode::Directory(subdir) => {
                    self.search_by_tag_recursive(subdir, tag, results);
                }
                _ => {}
            }
        }
    }

    
    pub fn search_by_mime_type(&self, mime_type: &str) -> Result<Vec<String>, String> {
        let mut results = Vec::new();
        self.search_by_mime_type_recursive(&self.root, mime_type, &mut results);
        Ok(results)
    }

    
    fn search_by_mime_type_recursive(
        &self,
        dir: &Directory,
        mime_type: &str,
        results: &mut Vec<String>,
    ) {
        for (name, node) in &dir.nodes {
            match node {
                FSNode::File(file) if file.metadata.mime_type == mime_type => {
                    results.push(format!("{}/{}", dir.name, file.name));
                }
                FSNode::Directory(subdir) => {
                    self.search_by_mime_type_recursive(subdir, mime_type, results);
                }
                _ => {}
            }
        }
    }
    pub fn rename(&mut self, old_path: &str, new_name: &str) -> Result<(), String> {
        let mut parts = old_path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>();
        if parts.is_empty() {
            return Err("Invalid path provided.".to_string());
        }

        let old_name = parts.pop().unwrap();
        let parent_dir = self.navigate_to_directory(&parts)?;

        if !parent_dir.nodes.contains_key(old_name) {
            return Err("File or directory not found.".to_string());
        }
        if parent_dir.nodes.contains_key(new_name) {
            return Err("A file or directory with the new name already exists.".to_string());
        }

        let node = parent_dir.nodes.remove(old_name).unwrap();
        parent_dir.nodes.insert(new_name.to_string(), node);

        Ok(())
    }

    pub fn copy(&mut self, source_path: &str, target_path: &str) -> Result<(), String> {
        let source_parts = source_path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>();
        let target_parts = target_path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>();
        let file_name = source_parts
            .last()
            .ok_or_else(|| "Invalid source path.".to_string())?;

        let (source_dir, _) = self.find_node(source_path)?;
        let node_to_clone = source_dir
            .nodes
            .get(*file_name)
            .ok_or_else(|| "Source file or directory not found.".to_string())?
            .clone();

        let target_dir = self.navigate_to_directory(&target_parts)?;
        target_dir
            .nodes
            .insert(file_name.to_string(), node_to_clone);

        Ok(())
    }

    pub fn get_info(&self, path: &str) -> Result<String, String> {
        let (dir, filename) = self.find_node(path)?;
        if let Some(node) = dir.nodes.get(filename) {
            let info = match node {
                FSNode::File(file) => format!(
                    "File Name: {}\nSize: {}\nPermissions: {:?}\nOwner: {}\nMIME Type: {}\nTags: {:?}",
                    file.name, file.metadata.size, file.metadata.permissions, file.metadata.owner, file.metadata.mime_type, file.metadata.tags
                ),
                FSNode::Directory(dir) => format!(
                    "Directory Name: {}\nSize: {}\nPermissions: {:?}\nOwner: {}",
                    dir.name, dir.metadata.size, dir.metadata.permissions, dir.metadata.owner
                ),
            };
            Ok(info)
        } else {
            Err("File or directory not found.".to_string())
        }
    }
}
impl Metadata {
    fn default() -> Self {
        let now = SystemTime::now();
        Metadata {
            created_at: now,
            modified_at: now,
            accessed_at: now,
            size: 0,
            permissions: Permissions {
                read: true,
                write: true,
                execute: false,
            },
            owner: "root".to_string(),
            group: "root".to_string(),
            is_read_only: false,
            is_hidden: false,
            mime_type: "text/plain".to_string(),
            tags: vec![],
        }
    }
}
impl FSNode {
    fn metadata(&mut self) -> &mut Metadata {
        match self {
            FSNode::File(file) => &mut file.metadata,
            FSNode::Directory(dir) => &mut dir.metadata,
        }
    }
}
