use anyhow::Result;
use clap::Parser;
use mesh_jawn::{radio::connection::connect_to_radio, utils::logging};

#[derive(Parser, Debug)]
#[command(name = "mesh-jawn", about, long_about = None, version)]
struct Args {
    #[arg(long, help = "Enable verbose (DEBUG) logging")]
    verbose: bool,

    #[arg(
        long,
        value_enum,
        default_value = "console",
        help = "Log format (console or json)"
    )]
    log_format: Option<LogFormat>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Run as interactive bot responding to commands
    Bot {
        #[arg(
            long,
            default_value = "nats://localhost:4222",
            help = "NATS server URL"
        )]
        nats_url: String,
    },
    /// Relay all packets to NATS JetStream
    Relay {
        #[arg(long, help = "Hostname or IP address of Meshtastic radio")]
        radio: String,

        #[arg(long, default_value = "4403", help = "TCP port for radio connection")]
        radio_port: u16,

        #[arg(
            long,
            default_value = "nats://localhost:4222",
            help = "NATS server URL"
        )]
        nats_url: String,

        #[arg(long, help = "Read-only mode: log instead of transmitting")]
        read_only: bool,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum LogFormat {
    Console,
    Json,
}

impl From<LogFormat> for logging::LogFormat {
    fn from(f: LogFormat) -> Self {
        match f {
            LogFormat::Console => logging::LogFormat::Console,
            LogFormat::Json => logging::LogFormat::Json,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    logging::configure_logging(args.verbose, args.log_format.map(Into::into));

    match args.command {
        Commands::Bot { nats_url } => {
            mesh_jawn::bot::run_bot(nats_url).await?;
        }
        Commands::Relay {
            radio,
            radio_port,
            nats_url,
            read_only,
        } => {
            let tcp_address = format!("{}:{}", radio, radio_port);

            let connection = connect_to_radio(tcp_address).await?;

            let config = mesh_jawn::relay::RelayConfig {
                nats_url,
                read_only,
            };

            mesh_jawn::relay::run_relay(
                connection.stream_api,
                connection.my_node_num,
                connection.my_node_info,
                connection.decoded_listener,
                config,
            )
            .await?;
        }
    }

    Ok(())
}
