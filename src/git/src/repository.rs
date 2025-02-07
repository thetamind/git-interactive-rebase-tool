use std::path::Path;

use anyhow::{anyhow, Result};

use crate::{commit_diff_loader::CommitDiffLoader, CommitDiff, CommitDiffLoaderOptions, Config};

/// A light simple wrapper around the `git2::Repository` struct
pub struct Repository {
	repository: git2::Repository,
}

impl Repository {
	/// Find and open an existing repository, respecting git environment variables. This will check
	/// for and use `$GIT_DIR`, and if unset will search for a repository starting in the current
	/// directory, walking to the root.
	///
	/// # Errors
	/// Will result in an error if the repository cannot be opened.
	#[inline]
	pub fn open_from_env() -> Result<Self> {
		let repository = git2::Repository::open_from_env()
			.map_err(|e| anyhow!(String::from(e.message())).context("Could not open repository from environment"))?;
		Ok(Self { repository })
	}

	/// Attempt to open an already-existing repository at `path`.
	///
	/// # Errors
	/// Will result in an error if the repository cannot be opened.
	#[inline]
	pub fn open_from_path(path: &Path) -> Result<Self> {
		let repository = git2::Repository::open(path)
			.map_err(|e| anyhow!(String::from(e.message())).context("Could not open repository from path"))?;
		Ok(Self { repository })
	}

	/// Load the git configuration for the repository.
	///
	/// # Errors
	/// Will result in an error if the configuration is invalid.
	#[inline]
	pub fn load_config(&self) -> Result<Config> {
		self.repository.config().map_err(|e| anyhow!(String::from(e.message())))
	}

	/// Load a diff for a commit hash
	///
	/// # Errors
	/// Will result in an error if the commit cannot be loaded.
	#[inline]
	pub fn load_commit_diff(&self, hash: &str, config: &CommitDiffLoaderOptions) -> Result<CommitDiff> {
		let oid = self.repository.revparse_single(hash)?.id();
		let loader = CommitDiffLoader::new(&self.repository, config);
		// TODO this is ugly because it assumes one parent
		Ok(loader.load_from_hash(oid).map_err(|e| anyhow!("{}", e))?.remove(0))
	}

	pub(crate) const fn git2_repository(&self) -> &git2::Repository {
		&self.repository
	}
}

impl From<git2::Repository> for Repository {
	#[inline]
	fn from(repository: git2::Repository) -> Self {
		Self { repository }
	}
}

impl ::std::fmt::Debug for Repository {
	#[inline]
	fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
		f.debug_struct("Repository")
			.field("[path]", &self.repository.path())
			.finish()
	}
}

// Paths in Windows makes these tests difficult, so disable
#[cfg(all(unix, test))]
mod tests {
	use std::env::set_var;

	use super::*;
	use crate::testutil::{with_temp_bare_repository, with_temp_repository};

	#[test]
	#[serial_test::serial]
	fn open_from_env() {
		let path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("test")
			.join("fixtures")
			.join("simple");
		set_var("GIT_DIR", path.to_str().unwrap());
		assert!(Repository::open_from_env().is_ok());
	}

	#[test]
	#[serial_test::serial]
	fn open_from_env_error() {
		let path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("test")
			.join("fixtures")
			.join("does-not-exist");
		set_var("GIT_DIR", path.to_str().unwrap());
		assert_eq!(
			format!("{:#}", Repository::open_from_env().err().unwrap()),
			format!(
				"Could not open repository from environment: failed to resolve path '{}': No such file or directory",
				path.to_str().unwrap()
			)
		);
	}

	#[test]
	fn open_from_path() {
		let path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("test")
			.join("fixtures")
			.join("simple");
		assert!(Repository::open_from_path(&path).is_ok());
	}

	#[test]
	fn open_from_path_error() {
		let path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("test")
			.join("fixtures")
			.join("does-not-exist");
		assert_eq!(
			format!("{:#}", Repository::open_from_path(&path).err().unwrap()),
			format!(
				"Could not open repository from path: failed to resolve path '{}': No such file or directory",
				path.to_str().unwrap()
			)
		);
	}

	#[test]
	fn load_config() {
		with_temp_bare_repository(|repo| {
			let _repo = repo.load_config()?;
			Ok(())
		});
	}

	#[test]
	fn load_commit_diff() {
		with_temp_repository(|repository| {
			let repo: git2::Repository = repository.repository;
			let id = {
				let tree = repo.find_tree(repo.index()?.write_tree()?)?;
				let sig = git2::Signature::new("name", "name@example.com", &git2::Time::new(1609459200, 0))?;
				let head = repo.find_reference("refs/heads/main")?.peel_to_commit()?;
				repo.commit(Some("HEAD"), &sig, &sig, "title", &tree, &[&head])?
			};
			let repository = Repository::from(repo);

			let _diff = repository
				.load_commit_diff(id.to_string().as_str(), &CommitDiffLoaderOptions::new())
				.unwrap();
			Ok(())
		});
	}

	#[test]
	fn from_git2_repository() {
		with_temp_bare_repository(|repository| {
			let repo: git2::Repository = repository.repository;
			let _repo = Repository::from(repo);
			Ok(())
		});
	}

	#[test]
	fn fmt() {
		with_temp_bare_repository(|repository| {
			let formatted = format!("{:?}", repository);
			let repo: git2::Repository = repository.repository;
			let path = repo.path().canonicalize().unwrap();
			assert_eq!(
				formatted,
				format!("Repository {{ [path]: \"{}/\" }}", path.to_str().unwrap())
			);
			Ok(())
		});
	}
}
