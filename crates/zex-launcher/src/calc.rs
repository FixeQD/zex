//! Arithmetic evaluation

/// Evaluate a mathematical expression and render the answer
pub fn evaluate(expression: &str) -> Option<String> {
    let mut context = fend_core::Context::new();
    let result = fend_core::evaluate(expression, &mut context).ok()?;
    Some(result.get_main_result().to_string())
}
