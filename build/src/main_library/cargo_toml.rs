use anyhow::Result;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::{error, instrument, trace};
use heck::ToUpperCamelCase;

use crate::icon_library::IconLibrary;

const BASE_CARGO_TOML: &str = indoc::indoc!(
    r#"
    # ------------------------------------------------------------------------------------------
    # THIS FILE WAS GENERATED BY THE "BUILD" CRATE.
    # ------------------------------------------------------------------------------------------

"#
);

#[derive(Debug)]
pub(crate) struct CargoToml {
    /// Path to the libraries Cargo.toml file.
    pub path: PathBuf,
}

impl CargoToml {
    #[instrument(level = "info")]
    async fn create_file(&mut self) -> Result<tokio::fs::File> {
        trace!("Creating file.");
        tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.path)
            .await
            .map_err(|err| {
                error!(?err, "Could not create file.");
                err
            })
            .map_err(Into::into)
    }

    #[instrument(level = "info")]
    pub(crate) async fn reset(&mut self) -> Result<()> {
        if self.path.exists() {
            trace!("Removing file.");
            tokio::fs::remove_file(&self.path).await?;
        }

        trace!("Writing BASE_CARGO_TOML content.");
        self.create_file()
            .await?
            .write_all(BASE_CARGO_TOML.as_bytes())
            .await
            .map_err(Into::into)
    }

    #[instrument(level = "info", skip_all)]
    async fn append(&mut self) -> Result<tokio::io::BufWriter<tokio::fs::File>> {
        trace!("Creating file.");
        Ok(tokio::io::BufWriter::new(
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&self.path)
                .await
                .map_err(|err| {
                    error!(?err, "Could not open file to append data.");
                    err
                })?,
        ))
    }

    pub async fn write_package_section(&mut self, lib_name: &str) -> Result<()> {
        let mut writer = self.append().await?;
        writer
            .write_all(
                indoc::indoc! {r#"
                [package]
                name = "{{package-name}}"
                version = "0.0.1"
                authors = ["Charles Edward Gagnon"]
                edition = "2021"
                description = "Icons library for the leptos web framework"
                readme = "./README.md"
                repository = "https://github.com/Carlosted/leptos-icons"
                license = "MIT"
                keywords = ["leptos", "icons"]
                categories = ["web-programming"]

            "#}
                .replace("{{package-name}}", lib_name)
                .as_bytes(),
            )
            .await?;
        writer.flush().await.map_err(|err| {
            error!(?err, "Could not flush Cargo.toml file after writing.");
            err
        })?;
        Ok(())
    }

    #[instrument(level = "info", skip(icon_libs))]
    pub(crate) async fn write_dependencies_section(
        &mut self,
        icon_libs: &[IconLibrary],
    ) -> Result<()> {
        let mut writer = self.append().await?;

        writer
            .write_all(
                indoc::indoc! {r#"
                [dependencies]
                leptos = { version = "0.2.5", default-features = false }
                leptos-icons-core = { path = "../leptos-icons-core" }
                serde = { version = "1", features = ["derive"], optional = true }

            "#}
                .as_bytes(),
            )
            .await?;

        for lib in icon_libs.iter() {
            writer
                // Example: leptos-icons-ai = { path = "../leptos-icons-ai" }
                .write_all(
                    format!(
                        "{lib_name} = {{  path = \"../{lib_name}\", optional = true }}\n",
                        lib_name = &lib.name
                    )
                    .as_bytes(),
                )
                .await?;
        }

        writer.write_all("\n".as_bytes()).await?;
        writer.flush().await.map_err(|err| {
            error!(?err, "Could not flush Cargo.toml file after writing.");
            err
        })?;

        Ok(())
    }

    #[instrument(level = "info", skip(icon_libs))]
    pub(crate) async fn write_features_section(&mut self, icon_libs: &[IconLibrary]) -> Result<()> {
        let mut writer = self.append().await?;

        writer
            .write_all(
                indoc::indoc! {r#"
                [features]
                serde = ["dep:serde"]

            "#}
                .as_bytes(),
            )
            .await?;

        for lib in icon_libs.iter() {
            writer
                // Example: Ai = []
                .write_all(
                    format!(
                        "{lib_short_name} = []\n",
                        lib_short_name = &lib.package.meta.short_name.to_upper_camel_case(),
                    )
                    .as_bytes(),
                )
                .await?;
        }

        for lib in icon_libs.iter() {
            for icon in &lib.icons {
                writer
                    // Example: AiPushpinTwotone = ["Ai", "leptos-icons-ai/AiPushpinTwotone"]
                    .write_all(
                        format!(
                            "{feature_name} = [\"{lib_short_name}\", \"{lib_name}/{feature_name}\"]\n",
                            lib_short_name = &lib.package.meta.short_name.to_upper_camel_case(),
                            lib_name = &lib.name,
                            feature_name = icon.feature.name,
                        )
                        .as_bytes(),
                    )
                    .await?;
            }
        }
        writer.flush().await.map_err(|err| {
            error!(?err, "Could not flush Cargo.toml file after writing.");
            err
        })?;

        Ok(())
    }
}
