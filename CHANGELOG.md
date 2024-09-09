# Changelog

All notable changes to this project will be documented in this file.

## [1.0.17] - 2024-09-09

### 🐛 Bug Fixes

- Duplicate changes entries and check if exist

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Update versions
- Update cliff signature

## [1.0.16] - 2024-08-07

### 🐛 Bug Fixes

- Change_exist iter was using any, changed to all

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Merge pull request #31 from websublime/fix/change-exist-any-to-all

## [1.0.15] - 2024-08-07

### 🚀 Features

- Snapshot version unique

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Merge pull request #30 from websublime/feature/snapshot-unique

## [1.0.14] - 2024-08-06

### 🐛 Bug Fixes

- Bump snapshot for dependencies when parent is snapshot too

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Merge pull request #29 from websublime/feature/dependency-patch-fix

## [1.0.13] - 2024-07-24

### 🚀 Features

- Remove PackageJsonSchema and safe parse only with serde

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Bump patch version
- Merge pull request #24 from websublime/feature/safe-parse-json

## [1.0.12] - 2024-07-24

### 🐛 Bug Fixes

- Change exist, should also check package is defined not only branch name

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Merge pull request #23 from websublime/fix/change-exist

## [1.0.11] - 2024-07-24

### 🚀 Features

- Changes pretty json and remove root package for pnpm

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Merge pull request #22 from websublime/feature/pretty-json-pnpm

## [1.0.10] - 2024-07-23

### 🚀 Features

- Apply bumps
- New git commands and sync dependencies implementation
- Apply_bumps ability to push changes

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Init sync dependencies
- Docs for get_bumps and apply_bumps
- Version maintenance
- Merge pull request #21 from websublime/feature/apply-changes

## [1.0.9] - 2024-07-22

### 🚀 Features

- Change file exist check
- Prepend to changelog

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Version maintenance
- Merge pull request #20 from websublime/feature/docs-and-change-file-exist

## [1.0.8] - 2024-07-19

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Make changes type available
- Merge pull request #19 from websublime/feature/changes-type

## [1.0.7] - 2024-07-19

### 🚀 Features

- Changes generation api
- Changes add exist function

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Merge pull request #18 from websublime/feature/changes
- Bump patch version

## [1.0.6] - 2024-07-19

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Change order for looking files
- Merge pull request #17 from websublime/feature/change

## [1.0.5] - 2024-07-19

### 🐛 Bug Fixes

- Avoid symbolic links and transforma paths to canonical usage

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Merge pull request #16 from websublime/fix/canonical-paths
- Bump version

## [1.0.4] - 2024-07-19

### 🐛 Bug Fixes

- Missing napi trait in bumps struct's

### ⚙️ Miscellaneous Tasks

- Maintenance changelog

## [1.0.3] - 2024-07-18

### 🐛 Bug Fixes

- Cfg for windows and exclude compile functions for testing

### ⚙️ Miscellaneous Tasks

- Changelog maintenance

## [1.0.2] - 2024-07-18

### 🐛 Bug Fixes

- Main to build correctly

### ⚙️ Miscellaneous Tasks

- Maintenance changelog

## [1.0.1] - 2024-07-18

### ⚙️ Miscellaneous Tasks

- Changelog maintenance
- Remove package random
- No verify enable
- Bump version

## [1.0.0] - 2024-07-18

### 🚀 Features

- Package manager and root project path
- Some git commands functions
- More git commands
- Remove clone method in git commands
- Get_packages for pnpm
- Get_packages for npm and yarn
- Get_packages for npm and yarn
- Get_changed_packages initial implementation
- Get_last_known_publish_tag_info_for_package init implementation
- Get_last_known_publish_tag_info_for_all_packages implementation
- Get_conventional_for_package implementation
- Init test structure
- Bumps implementation initialization
- Ge_bumps implementation

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Commits since test
- More git commands
- More git commands
- Maintenance format
- Init fixing tests
- Packages fixing tests
- Packages fixing tests
- Packages fixing tests
- Packages fixing tests
- Packages fixing tests
- Packages fixing tests
- Packages fixing tests
- Packages fixing tests
- Paths tests
- Maintenance format
- Test structure with git config
- Clippy all
- Packages module tests
- Conventional tests
- Serialize and deserialize on bump
- No trait clone in bump
- Missing cwd property
- Output debbug
- Conventional test find package
- Maintenance format
- Maintenance format
- Init apply_bumps
- Merge pull request #15 from websublime/feature/next
- Bump version

## [0.9.0] - 2024-07-12

### 🚀 Features

- Make napi and napi_derive optional features

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Maintenance format
- Documenting functions and methods
- Maintenance format
- Merge pull request #13 from websublime/feature/cfg-features

## [0.8.1] - 2024-07-11

### 🐛 Bug Fixes

- Package json validation changed to return boolean

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Maintenance version
- Merge pull request #12 from websublime/fix/package-validation

## [0.8.0] - 2024-07-11

### 🚀 Features

- Method to format repo url

### 🐛 Bug Fixes

- Clippy suggestions and upgrade serde

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Maintenance format
- Add repo url to package info
- Append to url for creation and init package json validation
- More options in package info
- Merge pull request #11 from websublime/feature/repo-url
- Bump version

## [0.7.1] - 2024-07-06

### 🐛 Bug Fixes

- Duplicated keys in changed packages

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Merge pull request #9 from websublime/fix/duplicated-changed-packages
- Bump version

## [0.7.0] - 2024-07-06

### 🚀 Features

- Monorepo method for getting changed packages

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Chang map tp flat_map
- Merge pull request #8 from websublime/feature/changed-packages
- Bump version

## [0.6.0] - 2024-07-06

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Add more git commands and fix git tag creation
- Maintenance format
- Fail safe for get_root_workspace by git
- Merge pull request #7 from websublime/feature/more-git
- Bump version

## [0.5.0] - 2024-07-04

### 🚀 Features

- Git commands
- Package info as now property for relative path of the package
- Package info converts to json and conventional config present in struct as json value
- Conventional package with commits,config and changelog output
- More git commands

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Maintenance sync lock file
- Maintenance format
- Support for changelog title
- Use of relative path from package info
- Maintenance format
- Serialize and deserialize objects
- Merge pull request #6 from websublime/feature/git
- Bump to version 0.5.0

## [0.4.0] - 2024-07-03

### 🚀 Features

- Monorepo struct api
- Package info struct with json value and tests to monorepo npm,pnpm
- Action CI to support pnpm

### 🐛 Bug Fixes

- Clippy issues

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Maintenance format
- Maintenance clippy suggestions
- Pnpm-lock.yaml
- Github action pnpm-lock creation and deletion
- Node without cache
- Merge pull request #5 from websublime/feature/monorepo
- Bump to version 0.4.0

## [0.3.0] - 2024-07-03

### 🚀 Features

- Project root detection

### 🐛 Bug Fixes

- Clippy checks
- Fmt formatting rules

### 🧪 Testing

- Add tests for filesystem root detection

### ⚙️ Miscellaneous Tasks

- Maintenance changelog
- Disable clippy checks
- Simplify match to unwrap_or_default
- Merge pull request #4 from websublime/feature/filesystem
- Bump to version 0.3.0

## [0.2.0] - 2024-07-03

### 🚀 Features

- Initial dependencies and configs
- Node package manager detection
- Agent display implementation

### ⚙️ Miscellaneous Tasks

- Init project
- Update github secret
- Maintenance format
- Init tests for agent
- Tests for all enum agents
- Tests for panic
- Add git-cliff config
- Merge pull request #2 from websublime/feature/agent
- Bump to version 0.2.0

<!-- generated by git-cliff -->
