use std::collections::HashMap;
use std::process::Command;



pub fn is_executor_present(executor: &str) -> bool {

    // add a mapping to get the executor name, if not in the map we use the parameter
    let alias_map: HashMap<&str, &str> = HashMap::from([
        ("psh", "powershell"),
    ]);
    let actual_executor = alias_map.get(executor).unwrap_or(&executor);

    Command::new(executor)
        .spawn()
        .map(|mut child| child.kill().is_ok()) // Kill immediately to avoid hanging.
        .unwrap_or(false)
}