use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub ultrabalancer_host: String,
    pub ultrabalancer_port: u16,
    pub prometheus_port: u16,
    pub grafana_port: u16,
    pub grafana_user: String,
    pub grafana_password: String,
    pub docker_network: String,
    pub project_name: String,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            ultrabalancer_host: "localhost".to_string(),
            ultrabalancer_port: 8080,
            prometheus_port: 9090,
            grafana_port: 3000,
            grafana_user: "admin".to_string(),
            grafana_password: generate_password(),
            docker_network: "ultrabalancer-net".to_string(),
            project_name: "ultrabalancer-dashboard".to_string(),
        }
    }
}

fn generate_password() -> String {
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*".chars().collect();
    let mut password = String::new();
    let mut rng = fastrand::Rng::new();
    for _ in 0..16 {
        password.push(chars[rng.usize(..chars.len())]);
    }
    password
}

pub struct DashboardManager;

impl DashboardManager {
    pub async fn interactive_setup() -> Result<DashboardConfig, anyhow::Error> {
        println!();
        println!("================================================================================");
        println!("                       ULTRA BALANCER DASHBOARD SETUP                          ");
        println!("================================================================================");
        println!();
        println!("  Deploy a complete monitoring stack with Grafana & Prometheus");
        println!();
        
        let mut config = DashboardConfig::default();
        
        println!("--------------------------------------------------------------------------------");
        println!("  ULTRABALANCER CONNECTION");
        println!("  Configure how Prometheus connects to UltraBalancer");
        println!("--------------------------------------------------------------------------------");
        config.ultrabalancer_host = prompt_input(
            "Enter UltraBalancer host",
            "localhost",
            &["localhost", "127.0.0.1", "0.0.0.0", "Custom IP or hostname"],
        );
        
        config.ultrabalancer_port = prompt_number(
            "Enter UltraBalancer port",
            8080,
            1,
            65535,
        );
        
        println!();
        println!("--------------------------------------------------------------------------------");
        println!("  SERVICE PORTS");
        println!("  Configure exposed ports for monitoring services");
        println!("--------------------------------------------------------------------------------");
        
        config.prometheus_port = prompt_number(
            "Prometheus port",
            9090,
            1,
            65535,
        );
        
        config.grafana_port = prompt_number(
            "Grafana port",
            3000,
            1,
            65535,
        );
        
        println!();
        println!("--------------------------------------------------------------------------------");
        println!("  SECURITY SETTINGS");
        println!("  Configure Grafana admin credentials");
        println!("--------------------------------------------------------------------------------");
        
        let use_custom = prompt_yes_no(
            "Use custom Grafana credentials?",
            false,
        );
        
        if use_custom {
            config.grafana_user = prompt_input(
                "Grafana username",
                "admin",
                &["Admin username"],
            );
            
            let mut new_pass;
            loop {
                new_pass = prompt_password("Grafana password", "Minimum 8 characters");
                let confirm = prompt_password("Confirm password", "Re-enter password");
                if new_pass == confirm && new_pass.len() >= 8 {
                    break;
                }
                println!("  ERROR: Passwords don't match or too short (min 8 chars)");
            }
            config.grafana_password = new_pass;
        }
        
        println!();
        println!("--------------------------------------------------------------------------------");
        println!("  DOCKER CONFIGURATION");
        println!("--------------------------------------------------------------------------------");
        
        let use_custom_network = prompt_yes_no(
            "Use custom Docker network name?",
            false,
        );
        
        if use_custom_network {
            config.docker_network = prompt_input(
                "Network name",
                "ultrabalancer-net",
                &["Docker network name"],
            );
        }
        
        let use_custom_project = prompt_yes_no(
            "Use custom project name?",
            false,
        );
        
        if use_custom_project {
            config.project_name = prompt_input(
                "Project name",
                "ultrabalancer-dashboard",
                &["Docker Compose project name"],
            );
        }
        
        println!();
        println!("================================================================================");
        println!("  DASHBOARD CONFIGURATION SUMMARY");
        println!("================================================================================");
        println!();
        println!("  UltraBalancer:  {}:{}", config.ultrabalancer_host, config.ultrabalancer_port);
        println!("  Prometheus:     localhost:{}", config.prometheus_port);
        println!("  Grafana:        localhost:{}", config.grafana_port);
        println!("  Grafana User:   {}", config.grafana_user);
        println!("  Grafana Pass:   {}", mask_password(&config.grafana_password));
        println!();
        
        let proceed = prompt_yes_no(
            "Start dashboard with these settings?",
            true,
        );
        
        if !proceed {
            println!();
            println!("  Dashboard setup cancelled. Run 'ultrabalancer dashboard' to try again.");
            std::process::exit(0);
        }
        
        Ok(config)
    }
    
    pub async fn generate_dashboard(config: &DashboardConfig, output_dir: &Path) -> Result<(), anyhow::Error> {
        println!();
        println!("  Generating dashboard files...");
        
        fs::create_dir_all(output_dir.join("provisioning/dashboards"))?;
        fs::create_dir_all(output_dir.join("provisioning/datasources"))?;
        
        let docker_compose = generate_docker_compose(config);
        let docker_path = output_dir.join("docker-compose.yml");
        fs::write(&docker_path, docker_compose)?;
        println!("  [OK] {}", docker_path.display());
        
        let prometheus_config = generate_prometheus_config(config);
        let prometheus_path = output_dir.join("prometheus.yml");
        fs::write(&prometheus_path, prometheus_config)?;
        println!("  [OK] {}", prometheus_path.display());
        
        let dashboard_json = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/grafana-dashboard.json")
        )?;
        let dashboard_path = output_dir.join("provisioning/dashboards/ultrabalancer-overview.json");
        fs::write(&dashboard_path, dashboard_json)?;
        println!("  [OK] {}", dashboard_path.display());
        
        let datasource_config = generate_grafana_datasource();
        let datasource_path = output_dir.join("provisioning/datasources/prometheus.yml");
        fs::write(&datasource_path, datasource_config)?;
        println!("  [OK] {}", datasource_path.display());
        
        let startup_script = generate_startup_script(config);
        let script_path = output_dir.join("start-dashboard.sh");
        fs::write(&script_path, startup_script)?;
        println!("  [OK] {}", script_path.display());
        
        let env_content = generate_env_file(config);
        let env_path = output_dir.join(".env");
        fs::write(&env_path, env_content)?;
        println!("  [OK] {}", env_path.display());
        
        Ok(())
    }
    
    pub async fn start_dashboard(config: &DashboardConfig, output_dir: &Path) -> Result<(), anyhow::Error> {
        println!();
        println!("  Starting Docker services...");
        
        std::env::set_current_dir(output_dir)?;
        
        println!("  Checking Docker availability...");
        let docker_check = std::process::Command::new("docker")
            .args(&["--version"])
            .output();
        
        if docker_check.is_err() {
            println!();
            println!("  ERROR: Docker is not installed or not running!");
            println!();
            println!("  Please install Docker:");
            println!("    - Ubuntu/Debian:  sudo apt-get install docker.io");
            println!("    - RHEL/CentOS:    sudo yum install docker");
            println!("    - macOS/Windows:  Download from https://docker.com");
            println!();
            return Ok(());
        }
        
        println!("  Creating Docker network...");
        std::process::Command::new("docker")
            .args(&["network", "create", "--driver", "bridge", &config.docker_network])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()?;
        
        println!("  Starting Prometheus on port {}...", config.prometheus_port);
        println!("  Starting Grafana on port {}...", config.grafana_port);
        
        let output = std::process::Command::new("docker")
            .args(&["compose", "-p", &config.project_name, "up", "-d", "--build"])
            .output()?;
        
        if !output.status.success() {
            println!("  ERROR: Failed to start Docker services");
            println!("{}", String::from_utf8_lossy(&output.stderr));
            return Ok(());
        }
        
        println!();
        println!("================================================================================");
        println!("  DASHBOARD IS READY!");
        println!("================================================================================");
        println!();
        
        let grafana_url = format!("http://localhost:{}", config.grafana_port);
        let prometheus_url = format!("http://localhost:{}", config.prometheus_port);
        
        println!("  +----------------------------------------------------------------------+");
        println!("  |                       YOUR MONITORING STACK                        |");
        println!("  +----------------------------------------------------------------------+");
        println!("  |  Grafana:     {:<55} |", grafana_url);
        println!("  |  Prometheus:  {:<55} |", prometheus_url);
        println!("  +----------------------------------------------------------------------+");
        println!("  |  Grafana Credentials:                                              |");
        println!("  |      User:     {:<55} |", config.grafana_user);
        println!("  |      Password: {:<55} |", mask_password(&config.grafana_password));
        println!("  +----------------------------------------------------------------------+");
        println!();
        
        let open_browser = prompt_yes_no("Open Grafana in browser?", true);
        
        if open_browser {
            println!();
            println!("  Opening Grafana...");
            
            #[cfg(target_os = "macos")]
            std::process::Command::new("open").arg(&grafana_url).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output()?;
            
            #[cfg(target_os = "linux")]
            std::process::Command::new("xdg-open").arg(&grafana_url).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output()?;
            
            #[cfg(target_os = "windows")]
            std::process::Command::new("cmd").args(&["/c", "start", &grafana_url]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output()?;
        }
        
        println!();
        println!("  NEXT STEPS:");
        println!("  1. Log into Grafana with credentials above");
        println!("  2. Dashboard 'UltraBalancer Overview' is auto-provisioned!");
        println!("  3. Prometheus datasource is auto-configured!");
        println!();
        println!("  To stop dashboard: cd dashboard && ./start-dashboard.sh stop");
        println!();
        
        Ok(())
    }
    
    pub async fn stop_dashboard(config: &DashboardConfig) -> Result<(), anyhow::Error> {
        println!();
        println!("  Stopping dashboard...");
        
        let output = std::process::Command::new("docker")
            .args(&["compose", "-p", &config.project_name, "down", "-v"])
            .output()?;
        
        if output.status.success() {
            std::process::Command::new("docker")
                .args(&["network", "rm", &config.docker_network])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .output()?;
            
            println!("  [OK] Dashboard stopped and cleaned up");
        } else {
            println!("  [ERROR] Failed to stop dashboard");
        }
        
        Ok(())
    }
    
    pub async fn restart_dashboard(config: &DashboardConfig, output_dir: &Path) -> Result<(), anyhow::Error> {
        println!();
        println!("  Restarting dashboard...");
        Self::stop_dashboard(config).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Self::start_dashboard(config, output_dir).await
    }
    
    pub async fn reset_dashboard(config: &DashboardConfig) -> Result<(), anyhow::Error> {
        println!();
        println!("  Resetting dashboard (stop + remove all data)...");
        
        Self::stop_dashboard(config).await?;
        
        let data_paths = &["prometheus_data", "grafana_data"];
        
        for path in data_paths {
            println!("  Removing volume {}...", path);
            std::process::Command::new("docker")
                .args(&["volume", "rm", &format!("{}-{}", config.project_name, path)])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .output()?;
        }
        
        println!("  [OK] Dashboard reset complete");
        Ok(())
    }
    
    pub async fn show_status(config: &DashboardConfig) -> Result<(), anyhow::Error> {
        println!();
        println!("================================================================================");
        println!("  DASHBOARD STATUS");
        println!("================================================================================");
        println!();
        
        let output = std::process::Command::new("docker")
            .args(&[
                "ps",
                "--format", "table {{.Names}}\t{{.Status}}\t{{.Ports}}",
                "--filter", &format!("name={}-prometheus", config.project_name),
                "--filter", &format!("name={}-grafana", config.project_name),
            ])
            .output()?;
        
        if output.status.success() {
            let status = String::from_utf8_lossy(&output.stdout);
            if status.contains("Up") {
                println!("  Services Running:");
                for line in status.lines() {
                    if !line.is_empty() && !line.contains("NAMES") {
                        println!("    - {}", line);
                    }
                }
            } else {
                println!("  Services not running");
                println!("  Run 'ultrabalancer dashboard start' to begin");
            }
        }
        
        println!();
        println!("  Grafana URL:     http://localhost:{}", config.grafana_port);
        println!("  Prometheus URL:  http://localhost:{}", config.prometheus_port);
        
        Ok(())
    }
    
    pub async fn edit_config(config: &DashboardConfig) -> Result<DashboardConfig, anyhow::Error> {
        println!();
        println!("================================================================================");
        println!("  EDIT DASHBOARD CONFIGURATION");
        println!("================================================================================");
        println!();
        
        let mut new_config = config.clone();
        
        println!("  Current: {}:{} (press Enter to keep)", config.ultrabalancer_host, config.ultrabalancer_port);
        new_config.ultrabalancer_host = prompt_input(
            "UltraBalancer host",
            &config.ultrabalancer_host,
            &[],
        );
        
        new_config.ultrabalancer_port = prompt_number(
            "UltraBalancer port",
            config.ultrabalancer_port,
            1,
            65535,
        );
        
        new_config.prometheus_port = prompt_number(
            "Prometheus port",
            config.prometheus_port,
            1,
            65535,
        );
        
        new_config.grafana_port = prompt_number(
            "Grafana port",
            config.grafana_port,
            1,
            65535,
        );
        
        println!();
        println!("  Configuration updated!");
        println!();
        
        let restart = prompt_yes_no("Restart dashboard with new settings?", true);
        
        if restart {
            println!();
            println!("  Restarting with new configuration...");
        }
        
        Ok(new_config)
    }
    
    pub async fn show_logs(config: &DashboardConfig) -> Result<(), anyhow::Error> {
        println!();
        println!("  Showing logs (Ctrl+C to exit)...");
        println!();
        
        let mut child = std::process::Command::new("docker")
            .args(&["compose", "-p", &config.project_name, "logs", "-f"])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;
        
        tokio::signal::ctrl_c().await?;
        let _ = child.kill();
        
        Ok(())
    }
}

fn prompt_input(prompt: &str, default: &str, _hints: &[&str]) -> String {
    println!("  {}", prompt);
    print!("  [{}] -> ", default);
    
    use std::io::{self, Write};
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    let input = input.trim();
    if input.is_empty() {
        default.to_string()
    } else {
        input.to_string()
    }
}

fn prompt_number(prompt: &str, default: u16, min: u16, max: u16) -> u16 {
    loop {
        println!("  {}", prompt);
        print!("  [{}] -> ", default);
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        let input = input.trim();
        if input.is_empty() {
            return default;
        }
        
        match input.parse::<u16>() {
            Ok(n) if n >= min && n <= max => return n,
            _ => {
                println!("  ERROR: Please enter a number between {} and {}", min, max);
            }
        }
    }
}

fn prompt_password(prompt: &str, hint: &str) -> String {
    println!("  {}", prompt);
    println!("  ({})", hint);
    print!("  -> ");
    
    use std::io::{self, Write};
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    input.trim().to_string()
}

fn prompt_yes_no(prompt: &str, default_yes: bool) -> bool {
    let default_str = if default_yes { "Y/n" } else { "y/N" };
    let default_bool = default_yes;
    
    loop {
        print!("  {} [{}] -> ", prompt, default_str);
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        let input = input.trim().to_lowercase();
        
        if input.is_empty() {
            return default_bool;
        }
        
        match input.chars().next().unwrap() {
            'y' => return true,
            'n' => return false,
            _ => {
                println!("  ERROR: Please enter 'y' or 'n'");
            }
        }
    }
}

fn mask_password(pwd: &str) -> String {
    let len = pwd.len();
    if len == 0 {
        "".to_string()
    } else if len <= 2 {
        "*".repeat(len)
    } else {
        format!("{}{}***", pwd.chars().next().unwrap(), pwd.chars().last().unwrap())
    }
}

fn generate_docker_compose(config: &DashboardConfig) -> String {
    format!(r#"version: '3.8'

services:
  prometheus:
    image: prom/prometheus:v2.48.0
    container_name: {project_name}-prometheus
    ports:
      - "{prometheus_port}:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.enable-lifecycle'
      - '--web.enable-admin-api'
    networks:
      - {network_name}
    restart: unless-stopped

  grafana:
    image: grafana/grafana:10.2.0
    container_name: {project_name}-grafana
    ports:
      - "{grafana_port}:3000"
    environment:
      - GF_SECURITY_ADMIN_USER={grafana_user}
      - GF_SECURITY_ADMIN_PASSWORD={grafana_password}
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - grafana_data:/var/lib/grafana
      - ./provisioning:/etc/grafana/provisioning:ro
    networks:
      - {network_name}
    restart: unless-stopped
    depends_on:
      - prometheus

volumes:
  prometheus_data:
  grafana_data:

networks:
  {network_name}:
    driver: bridge
"#,
        project_name = config.project_name,
        prometheus_port = config.prometheus_port,
        grafana_port = config.grafana_port,
        grafana_user = config.grafana_user,
        grafana_password = config.grafana_password,
        network_name = config.docker_network,
    )
}

fn generate_prometheus_config(config: &DashboardConfig) -> String {
    format!(r#"global:
  scrape_interval: 5s
  evaluation_interval: 5s

scrape_configs:
  - job_name: 'ultrabalancer'
    static_configs:
      - targets: ['{ultrabalancer_host}:{ultrabalancer_port}']
    metrics_path: /prometheus
    scheme: http
"#,
        ultrabalancer_host = config.ultrabalancer_host,
        ultrabalancer_port = config.ultrabalancer_port,
    )
}

fn generate_grafana_datasource() -> String {
    r#"apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    version: 1
    editable: false
"#.to_string()
}

fn generate_startup_script(config: &DashboardConfig) -> String {
    format!(r#"#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"
cd "$SCRIPT_DIR"

case "$1" in
    start)
        echo "Starting UltraBalancer Dashboard..."
        docker compose up -d --build
        echo "Dashboard started successfully!"
        echo "Grafana: http://localhost:{grafana_port}"
        echo "Prometheus: http://localhost:{prometheus_port}"
        ;;
    stop)
        echo "Stopping UltraBalancer Dashboard..."
        docker compose down -v
        docker network rm {network_name} 2>/dev/null || true
        echo "Dashboard stopped."
        ;;
    restart)
        echo "Restarting UltraBalancer Dashboard..."
        docker compose restart
        echo "Dashboard restarted."
        ;;
    status)
        echo "Checking dashboard status..."
        docker ps --filter "name={project_name}"
        ;;
    logs)
        docker compose logs -f "${{2:-}}"
        ;;
    *)
        echo "Usage: $0 {{start|stop|restart|status|logs}}"
        exit 1
        ;;
esac
"#,
        grafana_port = config.grafana_port,
        prometheus_port = config.prometheus_port,
        network_name = config.docker_network,
        project_name = config.project_name,
    )
}

fn generate_env_file(config: &DashboardConfig) -> String {
    format!(r#"ULTRABALANCER_HOST={ultrabalancer_host}
ULTRABALANCER_PORT={ultrabalancer_port}
PROMETHEUS_PORT={prometheus_port}
GRAFANA_PORT={grafana_port}
GRAFANA_USER={grafana_user}
GRAFANA_PASSWORD={grafana_password}
COMPOSE_PROJECT_NAME={project_name}
DOCKER_NETWORK={network_name}
"#,
        ultrabalancer_host = config.ultrabalancer_host,
        ultrabalancer_port = config.ultrabalancer_port,
        prometheus_port = config.prometheus_port,
        grafana_port = config.grafana_port,
        grafana_user = config.grafana_user,
        grafana_password = config.grafana_password,
        project_name = config.project_name,
        network_name = config.docker_network,
    )
}
