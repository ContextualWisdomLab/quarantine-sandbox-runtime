import unittest

from scripts.validate_repository import core_depends_on_application_service


class CoreDependencyValidationTests(unittest.TestCase):
    def test_comments_and_error_text_do_not_create_false_dependency(self) -> None:
        source = '''
//! Core sandbox execution.
/// The public alias `ApplicationServiceError` remains for compatibility.
// use crate::application_service::ApplicationServiceRequest;
#[error("ApplicationServiceError is compatibility text")]
pub enum SandboxRuntimeError {
    Failed,
}
'''

        self.assertFalse(core_depends_on_application_service(source))

    def test_supporting_module_path_is_rejected(self) -> None:
        source = "use crate::application_service::ApplicationServiceRequest;\n"

        self.assertTrue(core_depends_on_application_service(source))

    def test_supporting_type_reexport_is_rejected(self) -> None:
        source = "use crate::ApplicationServiceRequest;\n"

        self.assertTrue(core_depends_on_application_service(source))

    def test_nested_block_comments_do_not_hide_real_code_after_comment(self) -> None:
        source = '''
/* outer ApplicationServiceRequest
   /* nested crate::application_service::ApplicationServiceError */
*/
use crate::application_service::ApplicationServiceRequest;
'''

        self.assertTrue(core_depends_on_application_service(source))

    def test_raw_string_literal_with_embedded_quote_does_not_create_false_dependency(self) -> None:
        source = 'const MESSAGE: &str = r#"compatibility " ApplicationServiceError text"#;\n'

        self.assertFalse(core_depends_on_application_service(source))


if __name__ == "__main__":
    unittest.main()
