//! jq filter compilation and evaluation engine.
//!
//! Wraps [`jaq_core`] to provide a simple compile-then-run API for jq expressions.

use jaq_core::{data, unwrap_valr, Compiler, Ctx, Vars};
use jaq_core::load::{Arena, File, Loader};
use jaq_json::Val;

/// A compiled jq filter that can be run against JSON values.
pub struct JqEngine {
    filter: jaq_core::Filter<data::JustLut<Val>>,
}

impl JqEngine {
    /// Compile a jq filter string.
    pub fn compile(filter_str: &str) -> Result<Self, anyhow::Error> {
        let program = File { code: filter_str, path: () };
        let defs = jaq_core::defs().chain(jaq_std::defs());
        let funs = jaq_core::funs::<data::JustLut<Val>>().chain(jaq_std::funs());
        
        let loader = Loader::new(defs);
        let arena = Arena::default();
        let modules = loader.load(&arena, program).map_err(|errs| {
            anyhow::anyhow!("jq load error: {:?}", errs)
        })?;

        let filter = Compiler::default()
            .with_funs(funs)
            .compile(modules)
            .map_err(|errs| {
                anyhow::anyhow!("jq compile error: {:?}", errs)
            })?;

        Ok(Self { filter })
    }

    /// Run the compiled filter against a JSON value.
    pub fn run(&self, input: &serde_json::Value) -> Result<Vec<serde_json::Value>, anyhow::Error> {
        // Convert serde_json::Value to jaq_json::Val using serde feature
        let input_val: Val = serde_json::from_value(input.clone())
            .map_err(|e| anyhow::anyhow!("failed to convert json to jaq: {}", e))?;
        
        let ctx = Ctx::<data::JustLut<Val>>::new(&self.filter.lut, Vars::new([]));
        let out = self.filter.id.run((ctx, input_val)).map(unwrap_valr);
        
        let mut results = Vec::new();
        for output in out {
            match output {
                Ok(val) => {
                    let json_val: serde_json::Value = serde_json::from_str(&val.to_string())
                        .map_err(|e| anyhow::anyhow!("failed to parse jaq output as json: {}", e))?;
                    results.push(json_val);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("jq runtime error: {:?}", e));
                }
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compile_valid_filter() {
        let engine = JqEngine::compile(".foo");
        assert!(engine.is_ok());
    }

    #[test]
    fn compile_invalid_filter() {
        let engine = JqEngine::compile(".[invalid syntax!!");
        assert!(engine.is_err());
    }

    #[test]
    fn run_identity_filter() {
        let engine = JqEngine::compile(".").unwrap();
        let input = json!({"a": 1, "b": 2});
        let results = engine.run(&input).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], json!({"a": 1, "b": 2}));
    }

    #[test]
    fn run_field_access() {
        let engine = JqEngine::compile(".name").unwrap();
        let input = json!({"name": "pore", "version": "0.2.0"});
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!("pore")]);
    }

    #[test]
    fn run_array_filter() {
        let engine = JqEngine::compile("[.[] | select(. > 2)]").unwrap();
        let input = json!([1, 2, 3, 4, 5]);
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!([3, 4, 5])]);
    }

    #[test]
    fn run_multiple_outputs() {
        let engine = JqEngine::compile(".[]").unwrap();
        let input = json!([10, 20, 30]);
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!(10), json!(20), json!(30)]);
    }

    #[test]
    fn run_empty_output() {
        let engine = JqEngine::compile("empty").unwrap();
        let input = json!(null);
        let results = engine.run(&input).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn run_string_interpolation() {
        let engine = JqEngine::compile(r#""\(.a):\(.b)""#).unwrap();
        let input = json!({"a": "hello", "b": "world"});
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!("hello:world")]);
    }

    #[test]
    fn run_sort_by() {
        let engine = JqEngine::compile("sort_by(.x)").unwrap();
        let input = json!([{"x": 3}, {"x": 1}, {"x": 2}]);
        let results = engine.run(&input).unwrap();
        assert_eq!(results, vec![json!([{"x": 1}, {"x": 2}, {"x": 3}])]);
    }
}
