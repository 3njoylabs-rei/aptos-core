// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliError, MovePackageDir};
use crate::CliTypedResult;
use framework::natives::code::{ModuleMetadata, PackageMetadata, UpgradePolicy};
use move_deps::move_package::compilation::compiled_package::CompiledPackage;
use move_deps::move_package::BuildConfig;

/// Represents a built package on disk from which information can be extracted.
pub struct BuiltPackage {
    package_dir: MovePackageDir,
    package: CompiledPackage,
}

impl BuiltPackage {
    /// Builds the package and on success delivers a `BuiltPackage`.
    pub fn build(
        package_dir: MovePackageDir,
        generate_abis: bool,
        generate_docs: bool,
    ) -> CliTypedResult<Self> {
        let package_path = package_dir.get_package_path()?;
        let build_config = BuildConfig {
            additional_named_addresses: package_dir.named_addresses(),
            generate_abis,
            generate_docs,
            install_dir: package_dir.output_dir.clone(),
            ..Default::default()
        };
        let package = build_config
            .compile_package(&package_path, &mut Vec::new())
            .map_err(|err| CliError::MoveCompilationError(err.to_string()))?;
        Ok(Self {
            package_dir,
            package,
        })
    }

    /// Returns the name of this package.
    pub fn name(&self) -> &str {
        self.package.compiled_package_info.package_name.as_str()
    }

    /// Extracts the bytecode from the built package.
    pub fn extract_code(&self) -> Vec<Vec<u8>> {
        self.package
            .root_compiled_units
            .iter()
            .map(|unit_with_source| unit_with_source.unit.serialize(None))
            .collect()
    }

    /// Extracts metadata, as needed for publishing a package, from the built package.
    pub fn extract_metadata(
        &self,
        upgrade_policy: UpgradePolicy, // TODO: put this into Move.toml
    ) -> CliTypedResult<PackageMetadata> {
        let package_path = self.package_dir.get_package_path()?;

        let build_info = serde_yaml::to_string(&self.package.compiled_package_info)?;

        let manifest_file = package_path.join("Move.toml");
        let manifest = std::fs::read_to_string(&manifest_file)
            .map_err(|err| CliError::IO(manifest_file.display().to_string(), err))?;
        let mut modules = vec![];
        for u in &self.package.root_compiled_units {
            let name = u.unit.name().to_string();
            let source = std::fs::read_to_string(&u.source_path)
                .map_err(|err| CliError::IO(u.source_path.display().to_string(), err))?;
            let source_map = u.unit.serialize_source_map();
            let abi = if let Some(abis) = &self.package.compiled_abis {
                abis.iter()
                    .find(|(n, _)| n == &u.source_path.to_string_lossy().to_string())
                    .map(|(_, b)| b.clone())
                    .unwrap_or_default()
            } else {
                vec![]
            };
            modules.push(ModuleMetadata {
                name,
                source,
                source_map,
                abi,
            })
        }
        // TODO: need to build this on publish
        let error_map = vec![];

        Ok(PackageMetadata {
            name: self.name().to_string(),
            upgrade_policy,
            build_info,
            manifest,
            modules,
            error_map,
        })
    }
}
