# Test Suite Forge Package Manager

> Direktori ini memuat modul-modul pengujian integrasi dan sandbox testing untuk memverifikasi keandalan engine **`forge`** dan **`forge-server`**.

---

## 🧪 Struktur & Cakupan Pengujian

Total pengujian di Cargo Workspace terdiri dari **66 Unit & Integration Tests**:

### 1. Crate `crates/forge` (42 Tests):
- **CPU Profiler & Detection:** `test_cpu_detection`.
- **USE Flags & Slots Engine:** `test_use_flags_engine`.
- **DAG Dependency Resolver (Kahn Algorithm):** `test_dag_linear_resolution`, `test_dag_diamond_resolution`, `test_dag_cycle_detection`, `test_use_flags_conditional_filtering`, `test_meta_package_expansion`.
- **Sandbox Build Engine (Bubblewrap & Fallback):** `test_sandbox_command_construction`, `test_sandbox_fallback_execution`.
- **Transactional Merger & Collision Guard:** `test_preflight_collision_detector`, `test_atomic_merge_and_permissions`, `test_config_protect_mechanism`, `test_transactional_rollback_on_failure`.
- **Flat-File Manifest Database:** `test_manifest_entry_serialization`, `test_installed_database_record_and_get`, `test_unmerge_reverse_pruning`.
- **3-Tier Cascade & CachyOS Anti-Brick:** `test_cachyos_tier_auto_detection`, `test_core_os_blacklist_protection`, `test_parse_cachyos_desc_format`, `test_cachyos_recursive_dependency_chain`, `test_cachyos_recursive_respects_blacklist`, `test_parse_repo_db_tar_zst`, `test_cascade_prefers_forge_binhost`, `test_cascade_fallback_to_cachyos`, `test_cascade_blocks_core_os_from_cachyos`, `test_cascade_fallback_to_source`.
- **Upstream Recipe Importer:** `test_parse_pkgbuild_metadata`, `test_transpile_build_package_steps`, `test_dependency_normalization`, `test_validate_all_recipes_in_repo_are_valid_toml`.
- **Toolchain Bundler & Stage Exporter:** `test_toolchain_status_detection_including_glibc`, `test_bundle_seed_toolchain_usrmerge_structure_and_config`, `test_bundle_seed_toolchain_fails_when_empty_stage`, `test_stage_exporter_validates_usrmerge`, `test_stage_exporter_validates_openrc`, `test_stage_exporter_sanitizes_temporary_files`, `test_stage_exporter_full_export_tarball_and_checksums`, `test_stage_exporter_xz_format`.

### 2. Crate `crates/forge-server` (24 Tests):
- **Server State & Packaging:** `test_bundle_recipes_and_hash_generation`, `test_scan_packages_parses_all_fields`, `test_scan_actual_workspace_recipes`.
- **REST Endpoints & Explorer:** `test_server_health_and_endpoints`, `test_api_v1_packages_json_endpoint`, `test_index_html_endpoint_returns_html_and_contains_packages`, `test_binhost_catalog_and_package_endpoints`.
- **Client Sync Protocol:** `test_sync_recipes_client_full_cycle`, `test_sync_noop_when_up_to_date`.
- **GitOps Webhook & Auto-Rebundle:** `test_github_webhook_endpoint_triggers_rebundle`, `test_recipes_refresh_endpoint`.
- **Upstream Recipe Bumper & Audit:** `test_extract_github_repo`, `test_extract_tag_from_atom_feed`, `test_version_tag_cleaning`, `test_version_comparison`, `test_bump_recipe_toml_manipulation`.
- **Lock-CPU Profile Manager & CI/CD Builder:** `test_server_import_cpu_profile_saves_active`, `test_server_list_profiles`, `test_find_recipe_locations`, `test_server_build_uses_imported_active_profile`, `test_server_build_with_custom_profile_override`, `test_forge_server_build_and_import_separation`, `test_forge_server_import_registers_to_catalog`, `test_server_indexer_empty_and_populated`.

---

## 🚀 Menjalankan Pengujian

```bash
# Menjalankan seluruh 66 test suite secara paralel
cargo test --workspace

# Menjalankan pengujian crate spesifik
cargo test -p forge
cargo test -p forge-server
```
