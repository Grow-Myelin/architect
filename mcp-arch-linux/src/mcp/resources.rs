use super::Resource;

pub fn get_system_resources() -> Vec<Resource> {
    vec![
        Resource {
            uri: "system://info".to_string(),
            name: "System Information".to_string(),
            description: Some("Current system information and status".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        Resource {
            uri: "system://logs".to_string(),
            name: "System Logs".to_string(),
            description: Some("Recent system logs".to_string()),
            mime_type: Some("text/plain".to_string()),
        },
        Resource {
            uri: "system://services".to_string(),
            name: "Service Status".to_string(),
            description: Some("Status of system services".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        Resource {
            uri: "system://snapshots".to_string(),
            name: "System Snapshots".to_string(),
            description: Some("Available system snapshots for rollback".to_string()),
            mime_type: Some("application/json".to_string()),
        },
    ]
}