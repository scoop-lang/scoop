use std::collections::BTreeSet;

use scoop_manifest::ConeRelativePath;

use super::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    format: u32,
}

pub fn load_index(
    client: &RegistryClient<'_>,
    registry: &RegistryUrl,
    key: &DependencyCoordinateKey,
    offline: bool,
    refresh: bool,
) -> PmResult<RegistryIndex> {
    let cache = client.cache;
    let directory = cache.registry(registry);
    let path = directory
        .join("index")
        .join(key.group())
        .join(format!("{}.json", key.name()));
    let _guard = cache.lock("registry-index", format!("{registry}/{key}").as_bytes())?;
    if (!refresh || offline)
        && let Some(index) = read_json::<RegistryIndex>(&path)?
    {
        return Ok(index);
    }
    if offline {
        return Err(PmError::new(
            scoop_source::DiagnosticCode::E2112,
            format!("registry index for {key} at {registry} is not cached"),
        )
        .with_help(
            scoop_source::DiagnosticCode::H2101,
            "resolve without --offline".to_string(),
        ));
    }
    let config_path = directory.join("config.json");
    {
        let _config_guard = cache.lock("registry-config", registry.as_str().as_bytes())?;
        let config = read_json::<Config>(&config_path)?;
        let config = match config {
            Some(config) => config,
            None => {
                let config: Config =
                    download_json(&client.http, &format!("{}config.json", registry.as_str()))?;
                if config.format != 1 {
                    return Err(PmError::new(
                        scoop_source::DiagnosticCode::E2120,
                        format!("unsupported registry format {}", config.format),
                    ));
                }
                write_json(&config_path, &config)?;
                config
            }
        };
        if config.format != 1 {
            return Err(PmError::new(
                scoop_source::DiagnosticCode::E2120,
                format!("unsupported registry format {}", config.format),
            ));
        }
    }
    let url = format!(
        "{}packages/{}/{}.json",
        registry.as_str(),
        key.group(),
        key.name()
    );
    let index: RegistryIndex = download_json(&client.http, &url)?;
    index.validate(registry, key)?;
    write_json(&path, &index)?;
    Ok(index)
}

impl RegistryIndex {
    pub fn validate(&self, registry: &RegistryUrl, key: &DependencyCoordinateKey) -> PmResult<()> {
        if self.format != 1 || self.group != key.group() || self.name != key.name() {
            return Err(PmError::new(
                scoop_source::DiagnosticCode::E2120,
                format!("invalid registry index identity or format for {key} at {registry}"),
            ));
        }
        let mut versions = BTreeSet::new();
        for version in &self.versions {
            let coordinate = ConeCoordinate::new(&self.group, &self.name, &version.version)
                .map_err(|error| {
                    PmError::new(scoop_source::DiagnosticCode::E2010, error.to_string())
                })?;
            if !versions.insert(&version.version) {
                return Err(PmError::new(
                    scoop_source::DiagnosticCode::E2120,
                    format!("duplicate registry version {coordinate}"),
                ));
            }
            version.validate(registry, &coordinate)?;
        }
        Ok(())
    }
}

impl RegistryVersion {
    pub fn validate(&self, registry: &RegistryUrl, coordinate: &ConeCoordinate) -> PmResult<()> {
        archive_url(registry, &self.archive)?;
        if self.version != coordinate.version()
            || self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(PmError::new(
                scoop_source::DiagnosticCode::E2120,
                format!("invalid registry version or SHA-256 for {coordinate}"),
            ));
        }
        let mut keys = BTreeSet::new();
        for dependency in &self.dependencies {
            if !keys.insert(&dependency.key) {
                return Err(PmError::new(
                    scoop_source::DiagnosticCode::E2120,
                    format!(
                        "duplicate registry dependency {} in {coordinate}",
                        dependency.key
                    ),
                ));
            }
            match &dependency.source {
                RegistrySource::Sysroot if dependency.key.group() != "scoop" => {
                    return Err(PmError::new(
                        scoop_source::DiagnosticCode::E2011,
                        "sysroot dependency must use the scoop group",
                    ));
                }
                RegistrySource::Git {
                    git,
                    commit,
                    subdir,
                } => {
                    if git.is_empty()
                        || !matches!(commit.len(), 40 | 64)
                        || !commit
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                    {
                        return Err(PmError::new(
                            scoop_source::DiagnosticCode::E2120,
                            "registry Git dependency requires repository and full commit",
                        ));
                    }
                    ConeRelativePath::new(subdir).map_err(|message| {
                        PmError::new(scoop_source::DiagnosticCode::E2012, message)
                    })?;
                }
                _ => {}
            }
        }
        let core = ConeCoordinate::reserved_core();
        if *coordinate != core
            && !self.dependencies.iter().any(|dependency| {
                dependency.key == DependencyCoordinateKey::from_coordinate(&core)
                    && dependency
                        .version
                        .exact_version()
                        .is_some_and(|version| version.to_string() == core.version())
            })
        {
            return Err(PmError::new(
                scoop_source::DiagnosticCode::E2120,
                format!(
                    "registry metadata for {coordinate} is missing the exact implicit core edge"
                ),
            ));
        }
        Ok(())
    }
}

fn download_json<T: serde::de::DeserializeOwned>(http: &ureq::Agent, url: &str) -> PmResult<T> {
    let mut response = super::http::get(http, url)?;
    serde_json::from_reader(response.body_mut().as_reader()).map_err(|error| {
        PmError::new(
            scoop_source::DiagnosticCode::E2120,
            format!("{url}: {error}"),
        )
    })
}
