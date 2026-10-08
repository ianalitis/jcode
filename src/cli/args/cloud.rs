use clap::{Parser, Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub(crate) enum CloudCommand {
    /// Upload, list, verify, and view cloud-synced sessions
    Sessions {
        #[command(subcommand)]
        action: CloudSessionsCommand,
    },

    /// Move a live session (transcript, repo state, env notes) to a cloud host
    /// and keep working there. Your local files stay editable. Git reconciles
    /// both sides when the session comes back with `jcode cloud return`.
    Move {
        /// Session ID or name. Defaults to the session this command runs in.
        #[arg(long)]
        session: Option<String>,
        #[command(flatten)]
        target: CloudMoveTarget,
        /// Allow moving while the session is mid-turn (the agent itself runs /cloud).
        #[arg(long)]
        allow_active: bool,
        /// Prepare and verify only. Do not hand ownership to the cloud host.
        #[arg(long)]
        dry_run: bool,
        /// After a successful move, attach this terminal to the cloud session.
        #[arg(long)]
        attach: bool,
        #[arg(long)]
        json: bool,
    },

    /// Bring a moved session back: pull the cloud transcript and merge its git
    /// work into your local checkout with a normal 3-way merge.
    Return {
        #[arg(long)]
        session: Option<String>,
        #[command(flatten)]
        target: CloudMoveTarget,
        /// Bring the work back as refs only (refs/jcode-cloud/<session>/*) without touching the working tree.
        #[arg(long)]
        refs_only: bool,
        /// After returning, resume the session in this terminal.
        #[arg(long)]
        attach: bool,
        #[arg(long)]
        json: bool,
    },

    /// Show where moved sessions live and how their repos have diverged.
    Where {
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Attach this terminal to a session that lives on a cloud host.
    Attach {
        #[arg(long)]
        session: Option<String>,
    },

    /// Internal: remote side of `cloud move` (reads a bundle tarball on stdin).
    #[command(hide = true)]
    Receive {
        #[arg(long)]
        json: bool,
    },

    /// Internal: remote side of `cloud move` commit phase.
    #[command(hide = true)]
    Activate {
        #[arg(long)]
        session: String,
        #[arg(long)]
        epoch: u64,
    },

    /// Internal: remote side of `cloud return` (writes a bundle tarball to stdout).
    #[command(hide = true)]
    Export {
        #[arg(long)]
        session: String,
        #[arg(long)]
        epoch: u64,
    },
}

#[derive(Parser, Debug, Clone, Default)]
pub(crate) struct CloudMoveTarget {
    /// SSH host alias for the cloud machine. Defaults to [cloud] host in config
    /// or $JCODE_CLOUD_HOST.
    #[arg(long)]
    pub(crate) host: Option<String>,
    /// Remote jcode binary (name or path). Defaults to `jcode`.
    #[arg(long)]
    pub(crate) remote_binary: Option<String>,
    /// Local command used in place of `ssh <host>` (for tests and custom
    /// transports). Receives the remote shell command as its final argument.
    /// Also read from $JCODE_CLOUD_TRANSPORT.
    #[arg(long, hide = true)]
    pub(crate) transport: Option<String>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum CloudSessionsCommand {
    /// Configure Jade API defaults for cloud sessions on this machine
    Configure {
        /// Jade Session API base URL
        #[arg(long)]
        api_base: Option<String>,

        /// Jade Session API bearer token. Prefer --api-token-env to avoid shell history.
        #[arg(long, conflicts_with = "api_token_env")]
        api_token: Option<String>,

        /// Read the Jade Session API bearer token from this environment variable
        #[arg(long, conflicts_with = "api_token")]
        api_token_env: Option<String>,

        /// Optional Jade token id, e.g. dev-admin
        #[arg(long)]
        api_token_id: Option<String>,

        /// Default Jade user id for commands that do not pass --user-id
        #[arg(long)]
        user_id: Option<String>,

        /// Default private Jade session helper path
        #[arg(long)]
        helper: Option<String>,

        /// Remove the saved cloud sessions config
        #[arg(long)]
        clear: bool,
    },

    /// Show saved Jade API defaults for cloud sessions without printing secrets
    Status {
        /// Emit JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },

    /// Upload a specific local session JSON file to Jade cloud storage
    Upload {
        /// Path to a local Jcode session JSON file
        session_file: String,

        /// Upload without Jade's redaction pass
        #[arg(long)]
        raw: bool,

        #[command(flatten)]
        jade: JadeCloudOptions,
    },

    /// Upload the newest local Jcode session to Jade cloud storage
    UploadLatest {
        /// Directory containing local Jcode session JSON files
        #[arg(long, default_value = "~/.jcode/sessions")]
        sessions_dir: String,

        /// Upload without Jade's redaction pass
        #[arg(long)]
        raw: bool,

        #[command(flatten)]
        jade: JadeCloudOptions,
    },

    /// Sync new or changed local sessions to Jade cloud storage (idempotent; safe to schedule)
    Sync {
        /// Directory containing local Jcode session JSON files (default: ~/.jcode/sessions)
        #[arg(long)]
        sessions_dir: Option<String>,

        /// Only consider sessions modified within this many days (ignored with --all)
        #[arg(long)]
        since_days: Option<u64>,

        /// Sync all matching sessions regardless of age
        #[arg(long)]
        all: bool,

        /// Maximum number of sessions to upload in this run
        #[arg(long, default_value_t = 50)]
        max: usize,

        /// Skip this run if the last sync ran fewer than this many minutes ago (for cron/timers)
        #[arg(long)]
        min_interval_mins: Option<u64>,

        /// Upload without Jade's redaction pass
        #[arg(long)]
        raw: bool,

        /// Show what would be uploaded without uploading or recording state
        #[arg(long)]
        dry_run: bool,

        /// Re-upload sessions even if local sync state says they are unchanged
        #[arg(long)]
        force: bool,

        /// Emit JSON instead of human-readable text
        #[arg(long)]
        json: bool,

        #[command(flatten)]
        jade: JadeCloudOptions,
    },

    /// List cloud-uploaded sessions from the Jade index
    List {
        /// Maximum number of sessions to show
        #[arg(long, default_value_t = 25)]
        limit: usize,

        /// Emit JSON instead of human-readable text
        #[arg(long)]
        json: bool,

        #[command(flatten)]
        jade: JadeCloudOptions,
    },

    /// Verify that cloud metadata and the S3 session blob both exist
    Verify {
        /// Session ID to verify
        session_id: String,

        #[command(flatten)]
        jade: JadeCloudOptions,
    },

    /// Render a local HTML dashboard of cloud-uploaded sessions from the Jade index
    Dashboard {
        /// Maximum number of sessions to include
        #[arg(long, default_value_t = 100)]
        limit: usize,

        /// Write the dashboard HTML to this path (default: a temp file)
        #[arg(long)]
        output: Option<String>,

        /// Open the generated dashboard in the default browser
        #[arg(long)]
        open: bool,

        /// Also download each session and link rows to a local per-session viewer
        #[arg(long)]
        with_view: bool,

        #[command(flatten)]
        jade: JadeCloudOptions,
    },

    /// Download and view a cloud-uploaded session
    View {
        /// Session ID to view
        session_id: String,

        /// Output format
        #[arg(long, default_value = "summary")]
        format: CloudSessionViewFormat,

        /// Write HTML output to this path when --format html is used
        #[arg(long)]
        output: Option<String>,

        /// Open the generated HTML file when --format html is used
        #[arg(long)]
        open: bool,

        #[command(flatten)]
        jade: JadeCloudOptions,
    },
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct JadeCloudOptions {
    /// Jade user id to pass to the dev helper
    #[arg(long, default_value = "dev")]
    pub(crate) user_id: String,

    /// AWS CLI profile used by the private dev Jade helper. If omitted, the helper decides.
    #[arg(long)]
    pub(crate) profile: Option<String>,

    /// AWS region used by the private dev Jade helper. If omitted, the helper decides.
    #[arg(long)]
    pub(crate) region: Option<String>,

    /// Path to the private Jade session helper. Defaults to $JCODE_JADE_SESSIONS_HELPER or ~/jade/scripts/jade_sessions.py.
    #[arg(long)]
    pub(crate) helper: Option<String>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub(crate) enum CloudSessionViewFormat {
    Summary,
    Json,
    Html,
}

impl CloudSessionViewFormat {
    pub(crate) fn as_arg(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Json => "json",
            Self::Html => "html",
        }
    }
}
