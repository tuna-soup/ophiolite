use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::{MetadataCommand, Package};
use serde::Deserialize;

type BoxError = Box<dyn Error>;

#[derive(Debug, Clone, Deserialize)]
pub struct BoundaryManifest {
    #[serde(default)]
    pub class_rules: BTreeMap<String, ClassRule>,
    #[serde(default)]
    pub packages: BTreeMap<String, PackageBoundary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClassRule {
    #[serde(default)]
    pub allowed_dependencies: Vec<String>,
    #[serde(default)]
    pub forbidden_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageBoundary {
    pub class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackage {
    pub name: String,
    pub dependencies: Vec<WorkspaceDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkspaceDependency {
    pub name: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CheckReport {
    pub configuration_errors: Vec<String>,
    pub violations: Vec<Violation>,
}

impl CheckReport {
    pub fn is_empty(&self) -> bool {
        self.configuration_errors.is_empty() && self.violations.is_empty()
    }
}

impl fmt::Display for CheckReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut wrote_section = false;

        if !self.configuration_errors.is_empty() {
            wrote_section = true;
            writeln!(f, "Boundary configuration errors:")?;
            for error in &self.configuration_errors {
                writeln!(f, "- {error}")?;
            }
        }

        if !self.violations.is_empty() {
            if wrote_section {
                writeln!(f)?;
            }
            writeln!(f, "Boundary dependency violations:")?;
            for violation in &self.violations {
                writeln!(f, "- {violation}")?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    pub package_name: String,
    pub package_class: String,
    pub dependency_name: String,
    pub dependency_class: String,
    pub reason: ViolationReason,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.reason {
            ViolationReason::Forbidden => write!(
                f,
                "`{}` ({}) depends on `{}` ({}): class `{}` is forbidden for `{}`",
                self.package_name,
                self.package_class,
                self.dependency_name,
                self.dependency_class,
                self.dependency_class,
                self.package_class
            ),
            ViolationReason::NotAllowed => write!(
                f,
                "`{}` ({}) depends on `{}` ({}): class `{}` is not in the allowed dependency list for `{}`",
                self.package_name,
                self.package_class,
                self.dependency_name,
                self.dependency_class,
                self.dependency_class,
                self.package_class
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationReason {
    Forbidden,
    NotAllowed,
}

#[derive(Debug, Deserialize)]
struct RootManifest {
    workspace: WorkspaceSection,
}

#[derive(Debug, Deserialize)]
struct WorkspaceSection {
    metadata: WorkspaceMetadata,
}

#[derive(Debug, Deserialize)]
struct WorkspaceMetadata {
    ophiolite: OphioliteMetadata,
}

#[derive(Debug, Deserialize)]
struct OphioliteMetadata {
    boundaries: BoundaryManifest,
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("scripts/ophiolite-boundary-check should live two levels under repo root")
        .to_path_buf()
}

pub fn run_boundary_check(repo_root: &Path) -> Result<CheckReport, BoxError> {
    let boundaries = load_boundary_manifest(repo_root)?;
    let workspace_packages = load_workspace_packages(repo_root)?;
    Ok(check_workspace(&boundaries, &workspace_packages))
}

pub fn load_boundary_manifest(repo_root: &Path) -> Result<BoundaryManifest, BoxError> {
    let manifest_path = repo_root.join("Cargo.toml");
    let contents = fs::read_to_string(&manifest_path)?;
    let manifest: RootManifest = toml::from_str(&contents)?;
    Ok(manifest.workspace.metadata.ophiolite.boundaries)
}

pub fn load_workspace_packages(repo_root: &Path) -> Result<Vec<WorkspacePackage>, BoxError> {
    let manifest_path = repo_root.join("Cargo.toml");
    let metadata = MetadataCommand::new()
        .manifest_path(&manifest_path)
        .no_deps()
        .exec()?;

    let workspace_member_ids: BTreeSet<_> = metadata.workspace_members.into_iter().collect();
    let workspace_packages: Vec<_> = metadata
        .packages
        .into_iter()
        .filter(|package| workspace_member_ids.contains(&package.id))
        .collect();
    let workspace_names: BTreeSet<_> = workspace_packages
        .iter()
        .map(|package| package.name.clone())
        .collect();

    let mut packages = workspace_packages
        .into_iter()
        .map(|package| to_workspace_package(package, &workspace_names))
        .collect::<Vec<_>>();

    packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packages)
}

fn to_workspace_package(package: Package, workspace_names: &BTreeSet<String>) -> WorkspacePackage {
    let dependencies = package
        .dependencies
        .into_iter()
        .filter_map(|dependency| {
            workspace_names
                .contains(&dependency.name)
                .then_some(dependency.name)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|name| WorkspaceDependency { name })
        .collect();

    WorkspacePackage {
        name: package.name,
        dependencies,
    }
}

pub fn check_workspace(
    boundaries: &BoundaryManifest,
    packages: &[WorkspacePackage],
) -> CheckReport {
    let mut report = CheckReport::default();
    let declared_classes: BTreeSet<_> = boundaries.class_rules.keys().cloned().collect();

    for (class_name, rule) in &boundaries.class_rules {
        for referenced_class in rule
            .allowed_dependencies
            .iter()
            .chain(rule.forbidden_dependencies.iter())
        {
            if !declared_classes.contains(referenced_class) {
                report.configuration_errors.push(format!(
                    "class rule `{class_name}` references unknown dependency class `{referenced_class}`"
                ));
            }
        }
    }

    for (package_name, package_boundary) in &boundaries.packages {
        if !declared_classes.contains(&package_boundary.class) {
            report.configuration_errors.push(format!(
                "package `{package_name}` is assigned to unknown class `{}`",
                package_boundary.class
            ));
        }
    }

    for package in packages {
        let Some(package_boundary) = boundaries.packages.get(&package.name) else {
            report.configuration_errors.push(format!(
                "workspace package `{}` is missing from [workspace.metadata.ophiolite.boundaries.packages]",
                package.name
            ));
            continue;
        };

        let Some(rule) = boundaries.class_rules.get(&package_boundary.class) else {
            report.configuration_errors.push(format!(
                "workspace package `{}` uses undefined class `{}`",
                package.name, package_boundary.class
            ));
            continue;
        };

        let allowed_classes = rule.allowed_dependencies.iter().collect::<BTreeSet<_>>();
        let forbidden_classes = rule.forbidden_dependencies.iter().collect::<BTreeSet<_>>();

        for dependency in &package.dependencies {
            let Some(dependency_boundary) = boundaries.packages.get(&dependency.name) else {
                report.configuration_errors.push(format!(
                    "workspace dependency `{}` of `{}` is missing from [workspace.metadata.ophiolite.boundaries.packages]",
                    dependency.name, package.name
                ));
                continue;
            };

            if !boundaries
                .class_rules
                .contains_key(&dependency_boundary.class)
            {
                report.configuration_errors.push(format!(
                    "workspace dependency `{}` of `{}` uses undefined class `{}`",
                    dependency.name, package.name, dependency_boundary.class
                ));
                continue;
            }

            let reason = if forbidden_classes.contains(&dependency_boundary.class) {
                Some(ViolationReason::Forbidden)
            } else if !allowed_classes.contains(&dependency_boundary.class) {
                Some(ViolationReason::NotAllowed)
            } else {
                None
            };

            if let Some(reason) = reason {
                report.violations.push(Violation {
                    package_name: package.name.clone(),
                    package_class: package_boundary.class.clone(),
                    dependency_name: dependency.name.clone(),
                    dependency_class: dependency_boundary.class.clone(),
                    reason,
                });
            }
        }
    }

    report.configuration_errors.sort();
    report.configuration_errors.dedup();
    report.violations.sort();
    report.violations.dedup();
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_forbidden_dependency_classes() {
        let manifest = fixture_manifest(
            [
                ("platform_core", vec!["platform_core"], vec!["app_support"]),
                ("app_support", vec!["platform_core", "app_support"], vec![]),
            ],
            [("sdk", "platform_core"), ("app", "app_support")],
        );
        let packages = vec![
            fixture_package("sdk"),
            fixture_package("app"),
            WorkspacePackage {
                name: "sdk".to_string(),
                dependencies: vec![WorkspaceDependency {
                    name: "app".to_string(),
                }],
            },
        ];

        let report = check_workspace(&manifest, &packages);

        assert!(report.configuration_errors.is_empty());
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].reason, ViolationReason::Forbidden);
    }

    #[test]
    fn reports_not_allowed_dependency_classes() {
        let manifest = fixture_manifest(
            [
                (
                    "platform_support",
                    vec!["platform_support", "contract_shared"],
                    vec![],
                ),
                ("platform_core", vec!["platform_core"], vec![]),
                ("contract_shared", vec!["contract_shared"], vec![]),
            ],
            [("support", "platform_support"), ("core", "platform_core")],
        );
        let packages = vec![
            fixture_package("support"),
            fixture_package("core"),
            WorkspacePackage {
                name: "support".to_string(),
                dependencies: vec![WorkspaceDependency {
                    name: "core".to_string(),
                }],
            },
        ];

        let report = check_workspace(&manifest, &packages);

        assert!(report.configuration_errors.is_empty());
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].reason, ViolationReason::NotAllowed);
    }

    #[test]
    fn accepts_allowed_dependency_classes() {
        let manifest = fixture_manifest(
            [
                (
                    "platform_support",
                    vec!["platform_support", "contract_shared"],
                    vec![],
                ),
                ("contract_shared", vec!["contract_shared"], vec![]),
            ],
            [
                ("support", "platform_support"),
                ("contracts", "contract_shared"),
            ],
        );
        let packages = vec![
            fixture_package("support"),
            fixture_package("contracts"),
            WorkspacePackage {
                name: "support".to_string(),
                dependencies: vec![WorkspaceDependency {
                    name: "contracts".to_string(),
                }],
            },
        ];

        let report = check_workspace(&manifest, &packages);

        assert!(report.is_empty());
    }

    fn fixture_manifest<const RULES: usize, const PACKAGES: usize>(
        rules: [(&str, Vec<&str>, Vec<&str>); RULES],
        packages: [(&str, &str); PACKAGES],
    ) -> BoundaryManifest {
        let mut class_rules = BTreeMap::new();
        for (class_name, allowed_dependencies, forbidden_dependencies) in rules {
            class_rules.insert(
                class_name.to_string(),
                ClassRule {
                    allowed_dependencies: allowed_dependencies
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    forbidden_dependencies: forbidden_dependencies
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            );
        }

        let packages = packages
            .into_iter()
            .map(|(package_name, class_name)| {
                (
                    package_name.to_string(),
                    PackageBoundary {
                        class: class_name.to_string(),
                    },
                )
            })
            .collect();

        BoundaryManifest {
            class_rules,
            packages,
        }
    }

    fn fixture_package(name: &str) -> WorkspacePackage {
        WorkspacePackage {
            name: name.to_string(),
            dependencies: Vec::new(),
        }
    }
}
