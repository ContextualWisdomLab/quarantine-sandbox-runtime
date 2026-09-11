from pathlib import Path

cargo = Path("Cargo.toml")
cargo_text = cargo.read_text()
cargo_anchor = 'sha2 = "0.10"\nthiserror = "2.0"'
if cargo_text.count(cargo_anchor) != 1:
    raise SystemExit("unexpected Cargo dependency anchor")
cargo.write_text(
    cargo_text.replace(
        cargo_anchor,
        'sha2 = "0.10"\ntempfile = "=3.27.0"\nthiserror = "2.0"',
        1,
    )
)

source = Path("src/infrastructure/podman.rs")
text = source.read_text()
import_anchor = "    path::PathBuf,\n"
if text.count(import_anchor) != 1:
    raise SystemExit("unexpected path import anchor")
text = text.replace(import_anchor, "    path::{Path, PathBuf},\n", 1)

create_start = '        self.checked_output("network_create", plan.network_create_args())?;\n'
start = text.find(create_start)
if start < 0:
    raise SystemExit("application-service create start not found")
end_anchor = '        let start_args = ["start".to_owned(), container_id.clone()];\n'
end = text.find(end_anchor, start)
if end < 0:
    raise SystemExit("application-service create end not found")
new_create_block = '''        self.checked_output("network_create", plan.network_create_args())?;
        let create_receipt_directory = tempfile::Builder::new()
            .prefix("qsr-application-create-")
            .tempdir()
            .map_err(|_| ApplicationServiceError::BackendInvocationFailed {
                operation: "container_create_receipt",
            })?;
        let create_receipt_path = create_receipt_directory.path().join("container-id");
        let create_receipt_path_text = create_receipt_path.to_str().ok_or(
            ApplicationServiceError::BackendInvocationFailed {
                operation: "container_create_receipt",
            },
        )?;
        let mut create_args = plan.container_create_args().to_vec();
        create_args.insert(3, format!("--cidfile={create_receipt_path_text}"));

        let create_output = match self.checked_output("container_create", &create_args) {
            Ok(output) => output,
            Err(error) => {
                let receipt = match read_application_service_create_receipt(&create_receipt_path) {
                    Ok(receipt) => receipt,
                    Err(receipt_error) => {
                        self.cleanup_network(&plan)?;
                        return Err(receipt_error);
                    }
                };
                if let Some(container_id) = receipt {
                    self.cleanup_acquired_container(&plan, &container_id)?;
                } else {
                    self.cleanup_network(&plan)?;
                }
                return Err(error);
            }
        };
        let stdout_container_id = match parse_backend_identifier(&create_output.stdout) {
            Some(identifier) => identifier,
            None => {
                let original = ApplicationServiceError::MalformedIsolationInspection {
                    operation: "container_create",
                };
                let receipt = match read_application_service_create_receipt(&create_receipt_path) {
                    Ok(receipt) => receipt,
                    Err(receipt_error) => {
                        self.cleanup_network(&plan)?;
                        return Err(receipt_error);
                    }
                };
                if let Some(container_id) = receipt {
                    self.cleanup_acquired_container(&plan, &container_id)?;
                    return Err(original);
                }
                self.cleanup_network(&plan)?;
                return Err(ApplicationServiceError::MalformedIsolationInspection {
                    operation: "container_create_receipt",
                });
            }
        };
        let container_id = match read_application_service_create_receipt(&create_receipt_path) {
            Ok(Some(receipt_container_id)) if receipt_container_id == stdout_container_id => {
                receipt_container_id
            }
            Ok(Some(receipt_container_id)) => {
                self.cleanup_acquired_container(&plan, &receipt_container_id)?;
                return Err(ApplicationServiceError::MalformedIsolationInspection {
                    operation: "container_create_receipt",
                });
            }
            Ok(None) => {
                self.cleanup_acquired_container(&plan, &stdout_container_id)?;
                return Err(ApplicationServiceError::MalformedIsolationInspection {
                    operation: "container_create_receipt",
                });
            }
            Err(receipt_error) => {
                self.cleanup_acquired_container(&plan, &stdout_container_id)?;
                return Err(receipt_error);
            }
        };

'''
text = text[:start] + new_create_block + text[end:]

cleanup_start = text.find("    fn cleanup_created_container(\n")
cleanup_end = text.find("    fn cleanup_acquired_container(\n", cleanup_start)
if cleanup_start < 0 or cleanup_end < 0:
    raise SystemExit("generated-name cleanup helper not found")
text = text[:cleanup_start] + text[cleanup_end:]

helper_anchor = "fn runtime_identity() -> Result<String, ApplicationServiceError> {\n"
if text.count(helper_anchor) != 1:
    raise SystemExit("runtime identity helper anchor not unique")
helper = '''fn read_application_service_create_receipt(
    path: &Path,
) -> Result<Option<String>, ApplicationServiceError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(ApplicationServiceError::BackendInvocationFailed {
                operation: "container_create_receipt",
            });
        }
    };
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(_) => return Ok(None),
    };
    let identifier = text.strip_suffix('\n').unwrap_or(text);
    if identifier.len() != 64
        || !identifier
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Ok(None);
    }
    Ok(Some(identifier.to_owned()))
}

'''
text = text.replace(helper_anchor, helper + helper_anchor, 1)
source.write_text(text)
