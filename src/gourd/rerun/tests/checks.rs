use std::time::Duration;

use gourd_lib::config::slurm::ResourceLimits;

use super::*;

#[test]
fn query_update_resource_limits_script() {
    let limits = ResourceLimits {
        time_limit: Duration::from_secs(50326),
        mem_per_cpu: 13,
        cpus: 32,
    };

    let result = query_update_resource_limits(
        &limits, true, None, // mem
        None, // cpu
        None, // time
    );

    // No script mode, giving error
    assert!(result.is_err_and(|e| e.root_cause().to_string().contains("No time specified")));

    let result = query_update_resource_limits(
        &limits,
        true,
        None,                            // mem
        None,                            // cpu
        Some(Duration::from_secs(3000)), // time
    );

    // No script mode, giving error message
    assert!(result.is_err_and(|e| e.root_cause().to_string().contains("No memory specified")));

    let result = query_update_resource_limits(
        &limits,
        true,
        Some(283),                       // mem
        None,                            // cpu
        Some(Duration::from_secs(3000)), // time
    );

    // No script mode, giving error message
    assert!(result.is_err_and(|e| e.root_cause().to_string().contains("No CPUs specified")));

    let result = query_update_resource_limits(
        &limits,
        true,
        Some(283),                       // mem
        Some(38),                        // cpu
        Some(Duration::from_secs(3000)), // time
    );

    // No script mode, giving error message
    assert!(
        result.is_ok_and(|r| r.time_limit == Duration::from_secs(3000)
            && r.mem_per_cpu == 283
            && r.cpus == 38)
    );
}
