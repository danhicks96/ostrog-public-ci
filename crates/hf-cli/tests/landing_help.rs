//! The opening help describes the place the founders reach without assuming a landform.

use std::process::Command;

#[test]
fn opening_help_describes_the_egress_point() {
    for command in ["run", "soak"] {
        let output = Command::new(env!("CARGO_BIN_EXE_ostrog"))
            .args([command, "--help"])
            .output()
            .expect("run the built command-line executable");
        assert!(output.status.success(), "{command} --help failed");
        let help = String::from_utf8(output.stdout).expect("help is UTF-8");
        let normalized = help.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(normalized.contains("--opening"), "{command}: {help}");
        assert!(normalized.contains("egress point"), "{command}: {help}");
        assert!(normalized.contains("portal"), "{command}: {help}");
        assert!(
            !normalized.to_lowercase().contains("landing site"),
            "{command} help uses the retired name: {help}"
        );
        assert!(
            !normalized.to_lowercase().contains("valley"),
            "{command} help assumes the site is a valley: {help}"
        );
    }
}
