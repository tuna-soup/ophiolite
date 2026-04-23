use std::fs;
use std::path::{Path, PathBuf};

use seis_contracts_operations::import_ops::{SegyImportRecipe, SegyImportRecipeScope};

use crate::processing::unix_timestamp_s;

pub struct SegyImportRecipeState {
    recipes_dir: PathBuf,
}

impl SegyImportRecipeState {
    pub fn initialize(recipes_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(recipes_dir).map_err(|error| error.to_string())?;
        Ok(Self {
            recipes_dir: recipes_dir.to_path_buf(),
        })
    }

    pub fn list_recipes(
        &self,
        source_fingerprint: Option<&str>,
    ) -> Result<Vec<SegyImportRecipe>, String> {
        let mut recipes = Vec::new();
        for entry in fs::read_dir(&self.recipes_dir).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let recipe = serde_json::from_slice::<SegyImportRecipe>(
                &fs::read(&path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            if let Some(source_fingerprint) = source_fingerprint {
                if recipe.scope == SegyImportRecipeScope::SourceFingerprint
                    && recipe.source_fingerprint.as_deref() != Some(source_fingerprint)
                {
                    continue;
                }
            }
            recipes.push(recipe);
        }
        recipes.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.recipe_id.cmp(&right.recipe_id))
        });
        Ok(recipes)
    }

    pub fn save_recipe(&self, recipe: SegyImportRecipe) -> Result<SegyImportRecipe, String> {
        if recipe.recipe_id.trim().is_empty() {
            return Err("SEG-Y import recipe id must not be empty.".to_string());
        }
        if recipe.name.trim().is_empty() {
            return Err("SEG-Y import recipe name must not be empty.".to_string());
        }
        if recipe.scope == SegyImportRecipeScope::SourceFingerprint
            && recipe
                .source_fingerprint
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
        {
            return Err(
                "Source-specific SEG-Y import recipes require a source fingerprint.".to_string(),
            );
        }

        let now = unix_timestamp_s();
        let normalized = SegyImportRecipe {
            recipe_id: recipe.recipe_id.trim().to_string(),
            name: recipe.name.trim().to_string(),
            source_fingerprint: recipe
                .source_fingerprint
                .and_then(|value| (!value.trim().is_empty()).then(|| value.trim().to_string())),
            created_at_unix_s: if recipe.created_at_unix_s == 0 {
                now
            } else {
                recipe.created_at_unix_s
            },
            updated_at_unix_s: now,
            ..recipe
        };
        let path = self.recipe_path(&normalized.recipe_id);
        let json = serde_json::to_vec_pretty(&normalized).map_err(|error| error.to_string())?;
        fs::write(path, json).map_err(|error| error.to_string())?;
        Ok(normalized)
    }

    pub fn delete_recipe(&self, recipe_id: &str) -> Result<bool, String> {
        let path = self.recipe_path(recipe_id);
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(path).map_err(|error| error.to_string())?;
        Ok(true)
    }

    fn recipe_path(&self, recipe_id: &str) -> PathBuf {
        self.recipes_dir.join(format!("{recipe_id}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seis_contracts_operations::import_ops::{
        SegyGeometryOverride, SegyImportPlan, SegyImportPlanSource, SegyImportPolicy,
        SegyImportProvenance, SegyImportSparseHandling, SegyImportSpatialPlan,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("traceboost-segy-recipes-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn sample_recipe(
        scope: SegyImportRecipeScope,
        source_fingerprint: Option<&str>,
    ) -> SegyImportRecipe {
        SegyImportRecipe {
            recipe_id: "sample".to_string(),
            name: "Sample Recipe".to_string(),
            scope,
            source_fingerprint: source_fingerprint.map(str::to_string),
            plan: SegyImportPlan {
                input_path: "/tmp/input.sgy".to_string(),
                source_fingerprint: source_fingerprint.unwrap_or("source-a").to_string(),
                header_mapping: SegyGeometryOverride {
                    inline_3d: None,
                    crossline_3d: None,
                    third_axis: None,
                },
                spatial: SegyImportSpatialPlan {
                    x_field: None,
                    y_field: None,
                    coordinate_scalar_field: None,
                    coordinate_units: None,
                    coordinate_reference_id: None,
                    coordinate_reference_name: None,
                },
                policy: SegyImportPolicy {
                    sparse_handling: SegyImportSparseHandling::BlockImport,
                    output_store_path: "/tmp/output.tbvol".to_string(),
                    overwrite_existing: false,
                    acknowledge_warnings: false,
                },
                provenance: SegyImportProvenance {
                    plan_source: SegyImportPlanSource::Manual,
                    selected_candidate_id: None,
                    recipe_id: None,
                    recipe_name: None,
                },
            },
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
        }
    }

    #[test]
    fn recipe_state_filters_source_specific_entries() {
        let dir = unique_temp_dir();
        let state = SegyImportRecipeState::initialize(&dir).expect("init recipe state");

        state
            .save_recipe(sample_recipe(SegyImportRecipeScope::Global, None))
            .expect("save global recipe");
        state
            .save_recipe(SegyImportRecipe {
                recipe_id: "source-a".to_string(),
                ..sample_recipe(SegyImportRecipeScope::SourceFingerprint, Some("source-a"))
            })
            .expect("save source a recipe");
        state
            .save_recipe(SegyImportRecipe {
                recipe_id: "source-b".to_string(),
                ..sample_recipe(SegyImportRecipeScope::SourceFingerprint, Some("source-b"))
            })
            .expect("save source b recipe");

        let filtered = state
            .list_recipes(Some("source-a"))
            .expect("list filtered recipes");
        assert_eq!(filtered.len(), 2);
        assert!(
            filtered
                .iter()
                .any(|recipe| recipe.scope == SegyImportRecipeScope::Global)
        );
        assert!(
            filtered
                .iter()
                .any(|recipe| recipe.source_fingerprint.as_deref() == Some("source-a"))
        );

        fs::remove_dir_all(dir).expect("remove temp dir");
    }

    #[test]
    fn recipe_state_rejects_source_recipe_without_fingerprint() {
        let dir = unique_temp_dir();
        let state = SegyImportRecipeState::initialize(&dir).expect("init recipe state");
        let error = state
            .save_recipe(sample_recipe(
                SegyImportRecipeScope::SourceFingerprint,
                None,
            ))
            .expect_err("missing source fingerprint should fail");
        assert!(error.contains("source fingerprint"));
        fs::remove_dir_all(dir).expect("remove temp dir");
    }
}
