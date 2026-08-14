use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetManifest {
    pub files: Vec<String>,
}

impl AssetManifest {
    pub fn parse(json: &str) -> Result<Self, String> {
        let manifest: Self = serde_json::from_str(json)
            .map_err(|error| format!("Invalid asset manifest: {error}"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut paths = BTreeSet::new();
        for path in &self.files {
            let normalized = normalize_path(path)?;
            if !paths.insert(normalized.clone()) {
                return Err(format!("Duplicate manifest asset: {normalized}"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NamedTextAsset {
    pub path: String,
    pub contents: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NamedBinaryAsset {
    pub path: String,
    pub contents: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScenarioBundle {
    pub rhai_files: Vec<NamedTextAsset>,
    pub material_files: Vec<NamedBinaryAsset>,
    pub map_files: Vec<NamedBinaryAsset>,
    pub yarn_files: Vec<NamedTextAsset>,
}

impl ScenarioBundle {
    pub fn root_rhai(&self) -> Result<String, String> {
        let files = self.rhai_index()?;
        let mut output = String::new();
        let mut visiting = BTreeSet::new();
        let mut loaded = BTreeSet::new();
        // `web/scripts/pg_rpg.rhai` is the browser URL; inside a fetched
        // ScenarioBundle the web prefix is intentionally stripped.
        self.append_rhai(
            "scripts/pg_rpg.rhai",
            &files,
            &mut visiting,
            &mut loaded,
            &mut output,
        )?;
        Ok(output)
    }

    fn rhai_index(&self) -> Result<BTreeMap<String, String>, String> {
        let mut files = BTreeMap::new();
        for file in &self.rhai_files {
            let path = normalize_path(&file.path)?;
            if files.insert(path.clone(), file.contents.clone()).is_some() {
                return Err(format!("Duplicate Rhai file: {path}"));
            }
        }
        Ok(files)
    }

    fn append_rhai(
        &self,
        path: &str,
        files: &BTreeMap<String, String>,
        visiting: &mut BTreeSet<String>,
        loaded: &mut BTreeSet<String>,
        output: &mut String,
    ) -> Result<(), String> {
        if loaded.contains(path) {
            return Ok(());
        }
        if !visiting.insert(path.to_string()) {
            return Err(format!("Cyclic Rhai include: {path}"));
        }
        let contents = files
            .get(path)
            .ok_or_else(|| format!("Missing Rhai file: {path}"))?;
        for line in contents.lines() {
            if let Some(include) = line.trim().strip_prefix("// @include ") {
                let include = include.trim().trim_matches('"');
                let child = resolve_relative(path, include)?;
                self.append_rhai(&child, files, visiting, loaded, output)?;
            } else {
                output.push_str(line);
                output.push('\n');
            }
        }
        visiting.remove(path);
        loaded.insert(path.to_string());
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_asset_directories(root: &std::path::Path) -> Result<Self, String> {
        fn read_text_dir(dir: &std::path::Path) -> Result<Vec<NamedTextAsset>, String> {
            let mut paths = std::fs::read_dir(dir)
                .map_err(|error| format!("Cannot read {}: {error}", dir.display()))?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Cannot enumerate {}: {error}", dir.display()))?;
            paths.sort();
            paths
                .into_iter()
                .filter(|path| path.is_file())
                .map(|path| {
                    let contents = std::fs::read_to_string(&path)
                        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| format!("Invalid asset filename: {}", path.display()))?;
                    Ok(NamedTextAsset {
                        path: name.to_string(),
                        contents,
                    })
                })
                .collect()
        }

        let scripts = read_text_dir(&root.join("scripts"))?;
        Ok(Self {
            rhai_files: scripts
                .into_iter()
                .map(|mut file| {
                    file.path = format!("scripts/{}", file.path);
                    file
                })
                .collect(),
            ..Default::default()
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_web_directory(root: &std::path::Path) -> Result<Self, String> {
        fn read_manifest(root: &std::path::Path, directory: &str) -> Result<AssetManifest, String> {
            let path = root.join(directory).join("manifest.json");
            let json = std::fs::read_to_string(&path)
                .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
            AssetManifest::parse(&json)
        }

        fn read_text_assets(
            root: &std::path::Path,
            directory: &str,
            prefix: &str,
        ) -> Result<Vec<NamedTextAsset>, String> {
            let manifest = read_manifest(root, directory)?;
            manifest
                .files
                .into_iter()
                .map(|name| {
                    let path = root.join(directory).join(&name);
                    let contents = std::fs::read_to_string(&path)
                        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
                    Ok(NamedTextAsset {
                        path: format!("{prefix}{name}"),
                        contents,
                    })
                })
                .collect()
        }

        fn read_binary_assets(
            root: &std::path::Path,
            directory: &str,
            prefix: &str,
        ) -> Result<Vec<NamedBinaryAsset>, String> {
            let manifest = read_manifest(root, directory)?;
            manifest
                .files
                .into_iter()
                .map(|name| {
                    let path = root.join(directory).join(&name);
                    let contents = std::fs::read(&path)
                        .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
                    Ok(NamedBinaryAsset {
                        path: format!("{prefix}{name}"),
                        contents,
                    })
                })
                .collect()
        }

        Ok(Self {
            rhai_files: read_text_assets(root, "scripts", "scripts/")?,
            material_files: read_binary_assets(root, "assets/material", "materials/")?,
            map_files: read_binary_assets(root, "assets/map", "maps/")?,
            yarn_files: read_text_assets(root, "assets/yarnscript", "yarn/")?,
        })
    }
}

fn normalize_path(path: &str) -> Result<String, String> {
    let path = path.replace('\\', "/");
    if path.is_empty()
        || path.starts_with('/')
        || path.split('/').any(|part| part == ".." || part.is_empty())
    {
        return Err(format!("Invalid asset path: {path}"));
    }
    Ok(path)
}

fn resolve_relative(parent: &str, child: &str) -> Result<String, String> {
    let parent_dir = parent.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let path = if parent_dir.is_empty() {
        child.to_string()
    } else {
        format!("{parent_dir}/{child}")
    };
    normalize_path(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(path: &str, contents: &str) -> NamedTextAsset {
        NamedTextAsset {
            path: path.into(),
            contents: contents.into(),
        }
    }

    #[test]
    fn resolves_includes_deterministically() {
        let bundle = ScenarioBundle {
            rhai_files: vec![
                text(
                    "scripts/pg_rpg.rhai",
                    "// @include \"common.rhai\"\nlet root = 1;",
                ),
                text("scripts/common.rhai", "let common = 2;"),
            ],
            ..Default::default()
        };
        assert_eq!(
            bundle.root_rhai().unwrap(),
            "let common = 2;\nlet root = 1;\n"
        );
    }

    #[test]
    fn reports_missing_and_cyclic_files() {
        let missing = ScenarioBundle {
            rhai_files: vec![text("scripts/pg_rpg.rhai", "// @include \"missing.rhai\"")],
            ..Default::default()
        };
        assert!(
            missing
                .root_rhai()
                .unwrap_err()
                .contains("Missing Rhai file")
        );
        let cyclic = ScenarioBundle {
            rhai_files: vec![
                text("scripts/pg_rpg.rhai", "// @include \"a.rhai\""),
                text("scripts/a.rhai", "// @include \"pg_rpg.rhai\""),
            ],
            ..Default::default()
        };
        assert!(
            cyclic
                .root_rhai()
                .unwrap_err()
                .contains("Cyclic Rhai include")
        );
    }

    #[test]
    fn validates_manifests_and_rejects_duplicates() {
        let manifest = AssetManifest::parse(r#"{"files":["grass.toml","rock.toml"]}"#).unwrap();
        assert_eq!(manifest.files.len(), 2);
        let duplicate = AssetManifest::parse(r#"{"files":["grass.toml","grass.toml"]}"#);
        assert!(duplicate.unwrap_err().contains("Duplicate manifest asset"));
        let traversal = AssetManifest::parse(r#"{"files":["../secret.toml"]}"#);
        assert!(traversal.unwrap_err().contains("Invalid asset path"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn loads_the_runtime_pg_rpg_script_from_manifest() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
        let bundle = ScenarioBundle::from_web_directory(&root).unwrap();
        let script = bundle.root_rhai().unwrap();
        assert!(script.contains("scenario.add_secondary_job(1, \"Mage\")"));
        assert!(script.contains("tactical_grid.set_layer_bounds(0, 31)"));
    }
}
