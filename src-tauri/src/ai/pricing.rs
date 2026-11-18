//! Preço estimado por modelo (US$ por milhão de tokens). A Inworld não devolve o custo,
//! então o Lume estima pela tabela; modelo fora da tabela fica sem custo (só tokens).

/// (entrada, saída) em US$ por milhão de tokens.
pub fn price(model: &str) -> Option<(f64, f64)> {
    let name = model.rsplit('/').next().unwrap_or(model);
    Some(match name {
        "claude-opus-5-5" => (4.0, 20.0),
        "claude-sonnet-5-5" => (2.0, 10.0),
        "claude-haiku-5-5" => (0.10, 0.50),
        _ => return None,
    })
}

pub fn estimate(model: &str, input_tokens: i64, output_tokens: i64) -> Option<f64> {
    price(model).map(|(i, o)| (input_tokens as f64 * i + output_tokens as f64 * o) / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_model_has_cost() {
        let c = estimate("anthropic/claude-sonnet-5-5", 1_000_000, 100_000).unwrap();
        assert!((c - 3.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_has_no_cost() {
        assert_eq!(estimate("deepseek/deepseek-v4-flash", 10, 10), None);
    }
}
