use mdfmt::{
    FormatOptions,
    config::{ResolvedConfig, RuleConfig},
    format_with_config,
};
use serde_json::{Value, json};
use std::time::Instant;

fn summarize(samples: &[u64]) -> Value {
    assert!(!samples.is_empty());
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    json!({"median_us":sorted[sorted.len()/2],"p95_us":sorted[(sorted.len()*95).div_ceil(100)-1],"min_us":sorted[0],"max_us":sorted[sorted.len()-1]})
}

fn measure(name: &str, input: &str, count: usize) -> Value {
    let config = ResolvedConfig {
        rules: RuleConfig::from_value(
            json!({"default":false,"MD013":{"line_length":40,"strict":true}}),
        )
        .unwrap(),
        ..Default::default()
    };
    let expected = format_with_config(input, &FormatOptions::default(), &config).unwrap();
    let mut samples = vec![];
    for _ in 0..count {
        let start = Instant::now();
        let result = format_with_config(input, &FormatOptions::default(), &config).unwrap();
        samples.push(u64::try_from(start.elapsed().as_micros()).unwrap());
        assert_eq!(
            result.output, expected.output,
            "{name}: nondeterministic benchmark output"
        );
    }
    let mut row = summarize(&samples);
    row["category"] = json!(name);
    row["input_bytes"] = json!(input.len());
    row["output_bytes"] = json!(expected.output.len());
    row["changed"] = json!(expected.changed(input));
    row["remaining_findings"] = json!(expected.diagnostics.len());
    row["samples_us"] = json!(samples);
    row
}

fn main() {
    let references = format!(
        "{}\n{}",
        (0..128)
            .map(|index| format!("[reference {index}][id{index}] "))
            .collect::<String>(),
        (0..128)
            .map(|index| format!("[id{index}]: /path/{index}\n"))
            .collect::<String>()
    );
    let inputs = [
        (
            "prose",
            "alpha beta gamma delta epsilon zeta eta theta\n\n".repeat(256),
        ),
        (
            "lists",
            "- alpha beta gamma delta epsilon zeta eta theta\n".repeat(256),
        ),
        ("references", references),
        (
            "code",
            "```text\nalpha beta gamma delta epsilon zeta eta theta\n```\n\n".repeat(128),
        ),
        (
            "tables",
            format!(
                "| heading one | heading two |\n| --- | --- |\n{}",
                "| alpha beta gamma delta | epsilon zeta eta theta |\n".repeat(256)
            ),
        ),
    ];
    let rows: Vec<_> = inputs
        .iter()
        .map(|(name, input)| measure(name, input, 5))
        .collect();
    println!("{}",serde_json::to_string_pretty(&json!({"profile":if cfg!(debug_assertions){"debug"}else{"release"},"contract":"native library MD013 strict width 40; one warm-up and five independent formatting samples","results":rows})).unwrap());
}

#[cfg(test)]
mod tests {
    #[test]
    fn distributions_report_real_samples_and_order_statistics() {
        let summary = super::summarize(&[5, 1, 4, 2, 3]);
        assert_eq!(summary["median_us"], 3);
        assert_eq!(summary["p95_us"], 5);
        assert_eq!(summary["min_us"], 1);
        assert_eq!(summary["max_us"], 5);
        let row = super::measure(
            "prose",
            "alpha beta gamma delta epsilon zeta eta theta\n",
            5,
        );
        assert_eq!(row["samples_us"].as_array().unwrap().len(), 5);
        assert!(row["input_bytes"].as_u64().unwrap() > 0);
        assert!(row["changed"].as_bool().unwrap());
    }
}
