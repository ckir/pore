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
