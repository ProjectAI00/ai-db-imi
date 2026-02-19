use clap::{CommandFactory, Parser, Subcommand};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use std::env;
use std::fs::{self, File};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const VERSION: &str = "0.3.0";
// PostHog EU public capture key — safe to commit (client-side key)
const POSTHOG_KEY: &str = "phc_exyd1ppU0ZS7McQ1ay1gxvCaUI2QfYPuMCTk4kawVKF";

fn get_or_create_install_id(conn: &Connection) -> String {
    // Try to read existing install_id
    if let Ok(id) = conn.query_row(
        "SELECT value FROM settings WHERE key='install_id' LIMIT 1",
        [],
        |r| r.get::<_, String>(0),
    ) {
        return id;
    }
    // Generate a new one and persist it
    let id = format!("iid_{}", gen_id());
    let _ = conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('install_id', ?1)",
        params![id],
    );
    id
}

fn track(event: &str, install_id: &str, duration_ms: u128) {
    if env::var("IMI_NO_ANALYTICS").is_ok() {
        return;
    }
    if POSTHOG_KEY == "phc_REPLACE_ME" {
        return;
    }
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let body = format!(
        r#"{{"api_key":"{key}","event":"imi_{event}","distinct_id":"{id}","properties":{{"version":"{ver}","platform":"{plat}","duration_ms":{dur},"$lib":"imi-cli"}}}}"#,
        key = POSTHOG_KEY,
        event = event,
        id = install_id,
        ver = VERSION,
        plat = platform,
        dur = duration_ms,
    );
    let _ = Command::new("curl")
        .args([
            "-s", "-o", "/dev/null",
            "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", &body,
            "https://us.i.posthog.com/capture/",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn(); // fire-and-forget, never blocks
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Human,
    Toon,
    Json,
}

#[derive(Debug, Clone, Copy)]
struct OutputCtx {
    mode: OutputMode,
    color: bool,
}

impl OutputCtx {
    fn new(mode: OutputMode) -> Self {
        let color = matches!(mode, OutputMode::Human)
            && io::stdout().is_terminal()
            && env::var("TERM").unwrap_or_default() != "dumb";
        Self { mode, color }
    }

    fn is_toon(self) -> bool {
        matches!(self.mode, OutputMode::Toon)
    }

    fn is_json(self) -> bool {
        matches!(self.mode, OutputMode::Json)
    }
}

#[derive(Parser, Debug)]
#[command(name = "imi", version = VERSION, about = "IMI state engine", propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init,
    #[command(alias = "s")]
    Status,
    #[command(alias = "g")]
    Goals,
    #[command(alias = "t")]
    Tasks {
        filter: Option<String>,
    },
    #[command(alias = "ctx", alias = "c")]
    Context {
        goal_id: Option<String>,
    },
    #[command(alias = "n")]
    Next {
        #[arg(long)]
        agent: Option<String>,
        goal_id: Option<String>,
    },
    #[command(alias = "st")]
    Start {
        #[arg(long)]
        agent: Option<String>,
        task_id: String,
    },
    #[command(alias = "done")]
    Complete {
        #[arg(long)]
        agent: Option<String>,
        task_id: String,
        summary: Vec<String>,
    },
    Fail {
        #[arg(long)]
        agent: Option<String>,
        task_id: String,
        reason: Vec<String>,
    },
    Ping {
        task_id: String,
    },
    #[command(alias = "ag")]
    AddGoal {
        name: String,
        desc: Option<String>,
        priority: Option<String>,
        why: Option<String>,
        for_who: Option<String>,
        success_signal: Option<String>,
        #[arg(long, value_delimiter = ',')]
        relevant_files: Vec<String>,
        #[arg(long)]
        context: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
    },
    #[command(alias = "at")]
    AddTask {
        goal_id: String,
        title: String,
        desc: Option<String>,
        priority: Option<String>,
        why: Option<String>,
        #[arg(long)]
        context: Option<String>,
        #[arg(long, value_delimiter = ',')]
        relevant_files: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        tools: Vec<String>,
        #[arg(long)]
        acceptance_criteria: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
    },
    #[command(alias = "mem", alias = "m")]
    Memory {
        #[command(subcommand)]
        action: Option<MemoryAction>,
    },
    #[command(alias = "d")]
    Decide {
        what: String,
        why: String,
        affects: Option<String>,
    },
    #[command(alias = "l")]
    Log {
        note: Vec<String>,
    },
    #[command(alias = "rm")]
    Delete {
        id: String,
    },
    Reset {
        #[arg(short, long)]
        force: bool,
    },
    #[command(alias = "stat")]
    Stats,
    Instructions {
        target: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum MemoryAction {
    List,
    Add {
        goal_id: String,
        key: String,
        value: String,
    },
}

#[derive(Debug, Clone)]
struct GoalRow {
    id: String,
    name: String,
    description: String,
    why_: String,
    for_who: String,
    success_signal: String,
    status: String,
    priority: String,
    created_at: i64,
}

#[derive(Debug, Clone)]
struct TaskRow {
    id: String,
    title: String,
    description: String,
    why_: String,
    goal_id: Option<String>,
    status: String,
    priority: String,
    agent_id: Option<String>,
    created_at: i64,
}

#[derive(Debug, Clone)]
struct MemoryRow {
    id: String,
    goal_id: Option<String>,
    task_id: Option<String>,
    key: String,
    value: String,
    typ: String,
    source: String,
    created_at: i64,
}

#[derive(Debug, Clone)]
struct TaskClaim {
    id: String,
    title: String,
    description: String,
    why_: String,
    context: String,
    goal_id: Option<String>,
}

enum ClaimResult {
    NoTasks,
    RaceLost,
    Claimed(TaskClaim),
}

fn main() {
    let original_args: Vec<String> = env::args().collect();
    let (mode, parsed_args) = extract_output_mode(original_args);
    let out = OutputCtx::new(mode);

    let cli = match Cli::try_parse_from(parsed_args.clone()) {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return;
        }
    };

    let Some(command) = cli.command else {
        let mut cmd = Cli::command();
        let _ = cmd.print_help();
        println!();
        return;
    };

    let command_name = command_key(&command).to_string();
    let start = Instant::now();

    let mut db_path = match command {
        Commands::Init => {
            if let Ok(path) = env::var("IMI_DB") {
                if !path.trim().is_empty() {
                    PathBuf::from(path)
                } else {
                    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                    cwd.join(".imi").join("state.db")
                }
            } else {
                let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                cwd.join(".imi").join("state.db")
            }
        }
        _ => discover_db_path().unwrap_or_else(|| PathBuf::from(".imi/state.db")),
    };

    if let Ok(abs) = fs::canonicalize(&db_path) {
        db_path = abs;
    }

    let mut conn = match open_connection(&db_path) {
        Ok(c) => c,
        Err(e) => {
            emit_error(out, &e);
            std::process::exit(1);
        }
    };

    if let Err(e) = run_schema(&conn) {
        emit_error(out, &format!("schema error: {e}"));
        std::process::exit(1);
    }

    let result = dispatch(&mut conn, &db_path, out, command);

    let duration_ms = start.elapsed().as_millis();
    log_event(&conn, &command_name, None, None, None, duration_ms as i64);
    let install_id = get_or_create_install_id(&conn);
    track(&command_name, &install_id, duration_ms);

    if let Err(e) = result {
        emit_error(out, &e);
        std::process::exit(1);
    }
}

fn dispatch(conn: &mut Connection, db_path: &Path, out: OutputCtx, command: Commands) -> Result<(), String> {
    match command {
        Commands::Init => cmd_init(conn, db_path, out),
        Commands::Status => cmd_status(conn, db_path, out),
        Commands::Goals => cmd_goals(conn, out),
        Commands::Tasks { filter } => cmd_tasks(conn, out, filter),
        Commands::Context { goal_id } => cmd_context(conn, out, goal_id),
        Commands::Next { agent, goal_id } => cmd_next(conn, out, agent, goal_id),
        Commands::Start { agent, task_id } => cmd_start(conn, out, agent, task_id),
        Commands::Complete {
            agent,
            task_id,
            summary,
        } => cmd_complete(conn, out, agent, task_id, summary.join(" ")),
        Commands::Fail {
            agent,
            task_id,
            reason,
        } => cmd_fail(conn, out, agent, task_id, reason.join(" ")),
        Commands::Ping { task_id } => cmd_ping(conn, out, task_id),
        Commands::AddGoal {
            name,
            desc,
            priority,
            why,
            for_who,
            success_signal,
            relevant_files,
            context,
            workspace,
        } => cmd_add_goal(conn, out, name, desc, priority, why, for_who, success_signal, relevant_files, context, workspace),
        Commands::AddTask {
            goal_id,
            title,
            desc,
            priority,
            why,
            context,
            relevant_files,
            tools,
            acceptance_criteria,
            workspace,
        } => cmd_add_task(conn, out, goal_id, title, desc, priority, why, context, relevant_files, tools, acceptance_criteria, workspace),
        Commands::Memory { action } => cmd_memory(conn, out, action),
        Commands::Decide { what, why, affects } => cmd_decide(conn, out, what, why, affects),
        Commands::Log { note } => cmd_log(conn, out, note.join(" ")),
        Commands::Delete { id } => cmd_delete(conn, out, id),
        Commands::Reset { force } => cmd_reset(conn, out, force),
        Commands::Stats => cmd_stats(conn, out),
        Commands::Instructions { target } => cmd_instructions(out, target),
    }
}

fn extract_output_mode(args: Vec<String>) -> (OutputMode, Vec<String>) {
    let mut mode = OutputMode::Human;
    let mut keep = Vec::with_capacity(args.len());
    if let Some(first) = args.first() {
        keep.push(first.clone());
    }
    for arg in args.into_iter().skip(1) {
        match arg.as_str() {
            "--toon" => mode = OutputMode::Toon,
            "--json" => mode = OutputMode::Json,
            _ => keep.push(arg),
        }
    }
    (mode, keep)
}

fn command_key(command: &Commands) -> &'static str {
    match command {
        Commands::Init => "init",
        Commands::Status => "status",
        Commands::Goals => "goals",
        Commands::Tasks { .. } => "tasks",
        Commands::Context { .. } => "context",
        Commands::Next { .. } => "next",
        Commands::Start { .. } => "start",
        Commands::Complete { .. } => "complete",
        Commands::Fail { .. } => "fail",
        Commands::Ping { .. } => "ping",
        Commands::AddGoal { .. } => "add-goal",
        Commands::AddTask { .. } => "add-task",
        Commands::Memory { .. } => "memory",
        Commands::Decide { .. } => "decide",
        Commands::Log { .. } => "log",
        Commands::Delete { .. } => "delete",
        Commands::Reset { .. } => "reset",
        Commands::Stats => "stats",
        Commands::Instructions { .. } => "instructions",
    }
}

fn cmd_init(conn: &Connection, db_path: &Path, out: OutputCtx) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    let imi_dir = cwd.join(".imi");
    fs::create_dir_all(&imi_dir).map_err(|e| format!("failed to create .imi dir: {e}"))?;
    run_schema(conn)?;
    register_workspace(conn, &cwd)?;

    if out.is_json() {
        println!(
            "{}",
            json!({"ok": true, "db_path": db_path.display().to_string()})
        );
    } else if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "init",
            &["db_path"],
            vec![vec![db_path.display().to_string()]],
        );
        print!("{}", t.finish());
    } else {
        println!("● Initialized IMI in .imi/state.db");
    }
    Ok(())
}

fn cmd_status(conn: &Connection, db_path: &Path, out: OutputCtx) -> Result<(), String> {
    let goals_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM goals", [], |r| r.get(0))
        .unwrap_or(0);
    let (tasks_count, done_count, wip_count, review_count, todo_count): (i64, i64, i64, i64, i64) = conn
        .query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(CASE WHEN status='done' THEN 1 ELSE 0 END),0),
                    COALESCE(SUM(CASE WHEN status='in_progress' THEN 1 ELSE 0 END),0),
                    COALESCE(SUM(CASE WHEN status='review' THEN 1 ELSE 0 END),0),
                    COALESCE(SUM(CASE WHEN status='todo' THEN 1 ELSE 0 END),0)
             FROM tasks",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap_or((0, 0, 0, 0, 0));
    let memories_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
        .unwrap_or(0);

    let goals = get_goals(conn)?;

    if out.is_json() {
        let mut goal_json = Vec::new();
        for g in &goals {
            let tasks = get_tasks_for_goal(conn, &g.id)?;
            let total = tasks.len() as i64;
            let done = tasks.iter().filter(|t| t.status == "done").count() as i64;
            goal_json.push(json!({
                "id": g.id,
                "name": g.name,
                "status": g.status,
                "priority": g.priority,
                "done_tasks": done,
                "total_tasks": total,
                "tasks": tasks.iter().map(|t| json!({
                    "id": t.id,
                    "title": t.title,
                    "status": t.status,
                    "priority": t.priority,
                    "agent_id": t.agent_id
                })).collect::<Vec<_>>()
            }));
        }

        println!(
            "{}",
            json!({
                "version": VERSION,
                "db_path": db_path.display().to_string(),
                "counts": {
                    "goals": goals_count,
                    "tasks": tasks_count,
                    "done": done_count,
                    "wip": wip_count,
                    "review": review_count,
                    "todo": todo_count,
                    "memories": memories_count
                },
                "goals": goal_json
            })
        );
        return Ok(());
    }

    if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "counts",
            &["goals", "tasks", "done", "wip", "review", "todo", "memories"],
            vec![vec![
                goals_count.to_string(),
                tasks_count.to_string(),
                done_count.to_string(),
                wip_count.to_string(),
                review_count.to_string(),
                todo_count.to_string(),
                memories_count.to_string(),
            ]],
        );

        let mut goal_rows = Vec::new();
        let mut task_rows = Vec::new();
        for g in goals {
            let tasks = get_tasks_for_goal(conn, &g.id)?;
            let total = tasks.len();
            let done = tasks.iter().filter(|t| t.status == "done").count();
            goal_rows.push(vec![
                g.id.clone(),
                g.name.clone(),
                g.status.clone(),
                done.to_string(),
                total.to_string(),
            ]);
            for task in tasks {
                task_rows.push(vec![
                    g.id.clone(),
                    task.id,
                    task.title,
                    task.status,
                    task.priority,
                    task.agent_id.unwrap_or_default(),
                ]);
            }
        }
        t.section("goals", &["id", "name", "status", "done", "total"], goal_rows);
        t.section(
            "tasks",
            &["goal_id", "id", "title", "status", "priority", "agent"],
            task_rows,
        );
        print!("{}", t.finish());
        return Ok(());
    }

    println!(
        "{}",
        paint(out, "1", &format!("IMI State Engine  v{}", VERSION))
    );
    println!("DB: {}", db_path.display());
    println!();
    println!("  Goals       {}", goals_count);
    println!(
        "  Tasks       {}  {}{} done  {}{} wip  {}{} review  {}{} todo",
        tasks_count,
        status_icon(out, "done"),
        done_count,
        status_icon(out, "in_progress"),
        wip_count,
        status_icon(out, "review"),
        review_count,
        status_icon(out, "todo"),
        todo_count
    );
    println!("  Memories    {}", memories_count);
    println!();

    for g in get_goals(conn)? {
        let tasks = get_tasks_for_goal(conn, &g.id)?;
        let total = tasks.len();
        let done = tasks.iter().filter(|t| t.status == "done").count();
        println!(
            "  {} {} {}  ({}/{})  {}",
            status_icon(out, &g.status),
            priority_icon(out, &g.priority),
            g.name,
            done,
            total,
            g.id
        );
        for task in tasks {
            let agent = task.agent_id.unwrap_or_default();
            if agent.is_empty() {
                println!(
                    "    {} {} {}  {}",
                    status_icon(out, &task.status),
                    priority_icon(out, &task.priority),
                    task.title,
                    task.id
                );
            } else {
                println!(
                    "    {} {} {}  {}  @{}",
                    status_icon(out, &task.status),
                    priority_icon(out, &task.priority),
                    task.title,
                    task.id,
                    agent
                );
            }
        }
        println!();
    }

    Ok(())
}

fn cmd_goals(conn: &Connection, out: OutputCtx) -> Result<(), String> {
    let goals = get_goals(conn)?;

    if out.is_json() {
        println!(
            "{}",
            json!(goals
                .iter()
                .map(|g| json!({
                    "id": g.id,
                    "name": g.name,
                    "description": g.description,
                    "status": g.status,
                    "priority": g.priority,
                    "created_at": g.created_at,
                    "created_ago": ago(g.created_at)
                }))
                .collect::<Vec<_>>())
        );
        return Ok(());
    }

    if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "goals",
            &["id", "name", "status", "priority", "created_ago"],
            goals
                .iter()
                .map(|g| {
                    vec![
                        g.id.clone(),
                        g.name.clone(),
                        g.status.clone(),
                        g.priority.clone(),
                        ago(g.created_at),
                    ]
                })
                .collect(),
        );
        print!("{}", t.finish());
        return Ok(());
    }

    for g in goals {
        println!(
            "{} {} {}  {}  {}",
            status_icon(out, &g.status),
            priority_icon(out, &g.priority),
            g.id,
            g.name,
            ago(g.created_at)
        );
    }

    Ok(())
}

fn cmd_tasks(conn: &Connection, out: OutputCtx, filter: Option<String>) -> Result<(), String> {
    let filter = filter.unwrap_or_else(|| "all".to_string());
    let mut query = "SELECT t.id, t.title, COALESCE(t.description,''), COALESCE(t.why,''), t.goal_id, COALESCE(t.status,'todo'), COALESCE(t.priority,'medium'), t.agent_id, COALESCE(t.created_at,0), COALESCE(g.name,'') FROM tasks t LEFT JOIN goals g ON t.goal_id=g.id".to_string();
    let mut params_vec: Vec<String> = Vec::new();

    match filter.as_str() {
        "all" => {}
        "todo" | "done" | "review" => {
            query.push_str(" WHERE t.status=?1");
            params_vec.push(filter.clone());
        }
        "wip" | "in_progress" => {
            query.push_str(" WHERE t.status='in_progress'");
        }
        prefix => {
            let goal_id = resolve_id_prefix(conn, "goals", prefix)?
                .ok_or_else(|| format!("goal not found for prefix: {prefix}"))?;
            query.push_str(" WHERE t.goal_id=?1");
            params_vec.push(goal_id);
        }
    }
    query.push_str(" ORDER BY COALESCE(t.updated_at,t.created_at,0) DESC");

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = if params_vec.is_empty() {
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(9)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    } else {
        stmt.query_map(params![params_vec[0].clone()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(9)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };

    if out.is_json() {
        println!(
            "{}",
            json!(rows
                .iter()
                .map(|r| json!({"id": r.0, "title": r.1, "status": r.2, "priority": r.3, "goal": r.4}))
                .collect::<Vec<_>>())
        );
        return Ok(());
    }

    if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "tasks",
            &["id", "title", "status", "priority", "goal"],
            rows.iter()
                .map(|r| vec![r.0.clone(), r.1.clone(), r.2.clone(), r.3.clone(), r.4.clone()])
                .collect(),
        );
        print!("{}", t.finish());
        return Ok(());
    }

    for (id, title, status, priority, goal_name) in rows {
        if goal_name.is_empty() {
            println!(
                "{} {} {}  {}",
                status_icon(out, &status),
                priority_icon(out, &priority),
                id,
                title
            );
        } else {
            println!(
                "{} {} {}  {}  — {}",
                status_icon(out, &status),
                priority_icon(out, &priority),
                id,
                title,
                goal_name
            );
        }
    }
    Ok(())
}

fn cmd_context(conn: &Connection, out: OutputCtx, goal_id: Option<String>) -> Result<(), String> {
    if let Some(goal_prefix) = goal_id {
        return cmd_context_goal(conn, out, goal_prefix);
    }

    let now = now_ts();
    let week_ago = now - 7 * 24 * 3600;

    let direction = query_direction(conn, Some(week_ago), 10)?;
    let decisions = query_decisions(conn, 10)?;
    let active_goals = query_active_goals(conn, 10)?;
    let wip = query_wip_tasks(conn, 10)?;
    let memories = query_memories(conn, None, 10)?;

    if out.is_json() {
        let direction_json: Vec<Value> = direction
            .iter()
            .map(|d| json!({"content": d.0, "author": d.1, "created_at": d.2}))
            .collect();
        let decisions_json: Vec<Value> = decisions
            .iter()
            .map(|d| json!({"what": d.0, "why": d.1, "affects": d.2, "created_at": d.3}))
            .collect();
        let goals_json: Vec<Value> = active_goals.iter().map(goal_to_value).collect();
        let wip_json: Vec<Value> = wip.iter().map(wip_task_to_value).collect();
        let memories_json: Vec<Value> = memories.iter().map(memory_to_value).collect();
        println!(
            "{}",
            json!({
                "direction": direction_json,
                "decisions": decisions_json,
                "goals": goals_json,
                "wip": wip_json,
                "memories": memories_json
            })
        );
        return Ok(());
    }

    if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "direction",
            &["content", "author", "created_at"],
            direction
                .iter()
                .map(|d| vec![d.0.clone(), d.1.clone(), d.2.to_string()])
                .collect(),
        );
        t.section(
            "decisions",
            &["what", "why", "affects", "created_at"],
            decisions
                .iter()
                .map(|d| vec![d.0.clone(), d.1.clone(), d.2.clone(), d.3.to_string()])
                .collect(),
        );
        t.section(
            "goals",
            &["id", "name", "status", "priority"],
            active_goals
                .iter()
                .map(|g| vec![g.id.clone(), g.name.clone(), g.status.clone(), g.priority.clone()])
                .collect(),
        );
        t.section(
            "wip",
            &["id", "title", "goal", "agent"],
            wip.iter()
                .map(|w| {
                    vec![
                        w.id.clone(),
                        w.title.clone(),
                        w.goal_name.clone().unwrap_or_default(),
                        w.agent_id.clone().unwrap_or_default(),
                    ]
                })
                .collect(),
        );
        t.section(
            "memories",
            &["key", "value", "type", "source"],
            memories
                .iter()
                .map(|m| vec![m.key.clone(), m.value.clone(), m.typ.clone(), m.source.clone()])
                .collect(),
        );
        print!("{}", t.finish());
        return Ok(());
    }

    println!("IMI Context  what matters right now\n");

    println!("Direction notes (last 7 days)");
    if direction.is_empty() {
        println!("  (none)");
    } else {
        for d in &direction {
            let author = if d.1.is_empty() { "unknown" } else { &d.1 };
            println!("  ▸ {}\n    {} ago  @{}", d.0, ago(d.2), author);
        }
    }

    println!("\nDecisions");
    if decisions.is_empty() {
        println!("  (none)");
    } else {
        for d in &decisions {
            println!("  {}\n    why: {}\n    affects: {}\n    {} ago", d.0, d.1, d.2, ago(d.3));
        }
    }

    println!("\nActive Goals");
    if active_goals.is_empty() {
        println!("  (none)");
    } else {
        for g in &active_goals {
            println!(
                "  {} {} {}  {} ago",
                status_icon(out, &g.status),
                priority_icon(out, &g.priority),
                g.name,
                ago(g.created_at)
            );
            if !g.why_.is_empty() {
                println!("    why: {}", g.why_);
            }
        }
    }

    println!("\nIn Progress");
    if wip.is_empty() {
        println!("  (nothing in progress)");
    } else {
        for t in &wip {
            println!(
                "  {} {} {}  {}",
                status_icon(out, &t.status),
                priority_icon(out, &t.priority),
                t.title,
                t.id
            );
        }
    }

    Ok(())
}

fn cmd_context_goal(conn: &Connection, out: OutputCtx, goal_prefix: String) -> Result<(), String> {
    let goal_id = resolve_id_prefix(conn, "goals", &goal_prefix)?
        .ok_or_else(|| format!("goal not found: {goal_prefix}"))?;
    let goal = get_goal(conn, &goal_id)?.ok_or_else(|| "goal not found".to_string())?;
    let tasks = get_tasks_for_goal(conn, &goal_id)?;
    let memories = query_memories(conn, Some(&goal_id), 30)?;

    if out.is_json() {
        let tasks_json: Vec<Value> = tasks.iter().map(task_to_value).collect();
        let memories_json: Vec<Value> = memories.iter().map(memory_to_value).collect();
        println!(
            "{}",
            json!({
                "goal": goal_to_value(&goal),
                "tasks": tasks_json,
                "memories": memories_json
            })
        );
        return Ok(());
    }

    if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "goal",
            &["id", "name", "status", "why", "for_who", "success"],
            vec![vec![
                goal.id.clone(),
                goal.name.clone(),
                goal.status.clone(),
                goal.why_.clone(),
                goal.for_who.clone(),
                goal.success_signal.clone(),
            ]],
        );
        t.section(
            "tasks",
            &["id", "title", "status", "priority", "agent"],
            tasks
                .iter()
                .map(|x| {
                    vec![
                        x.id.clone(),
                        x.title.clone(),
                        x.status.clone(),
                        x.priority.clone(),
                        x.agent_id.clone().unwrap_or_default(),
                    ]
                })
                .collect(),
        );
        t.section(
            "memories",
            &["key", "value", "type", "source"],
            memories
                .iter()
                .map(|m| vec![m.key.clone(), m.value.clone(), m.typ.clone(), m.source.clone()])
                .collect(),
        );
        print!("{}", t.finish());
        return Ok(());
    }

    println!("{} {}  {}", status_icon(out, &goal.status), goal.name, goal.id);
    if !goal.why_.is_empty() {
        println!("why: {}", goal.why_);
    }
    if !goal.for_who.is_empty() {
        println!("for who: {}", goal.for_who);
    }
    if !goal.success_signal.is_empty() {
        println!("success: {}", goal.success_signal);
    }

    println!("\nTasks");
    if tasks.is_empty() {
        println!("  (none)");
    } else {
        for t in tasks {
            println!(
                "  {} {} {}  {}",
                status_icon(out, &t.status),
                priority_icon(out, &t.priority),
                t.title,
                t.id
            );
        }
    }

    println!("\nMemories");
    if memories.is_empty() {
        println!("  (none)");
    } else {
        for m in memories {
            println!("  [{}] {} = {}", m.typ, m.key, m.value);
        }
    }

    Ok(())
}

fn cmd_next(
    conn: &mut Connection,
    out: OutputCtx,
    agent: Option<String>,
    goal_prefix: Option<String>,
) -> Result<(), String> {
    let released = release_stale_locks(conn)?;
    let agent_id = current_agent(agent.as_deref());

    let goal_filter = if let Some(prefix) = goal_prefix {
        Some(
            resolve_id_prefix(conn, "goals", &prefix)?
                .ok_or_else(|| format!("goal not found: {prefix}"))?,
        )
    } else {
        None
    };

    match claim_next_task(conn, goal_filter.as_deref(), &agent_id)? {
        ClaimResult::NoTasks => {
            if out.is_json() {
                println!("{}", json!({"ok": true, "no_tasks": true, "released_stale": released}));
            } else if out.is_toon() {
                let mut t = ToonBuilder::new();
                t.section("no_tasks", &["note"], vec![vec!["all_done_or_claimed".to_string()]]);
                print!("{}", t.finish());
            } else {
                if released > 0 {
                    println!("⚠ Released {released} stale in-progress task(s)");
                }
                println!("No available tasks (all done or already claimed).");
            }
            Ok(())
        }
        ClaimResult::RaceLost => {
            if out.is_json() {
                println!("{}", json!({"ok": true, "race_lost": true}));
            } else if out.is_toon() {
                let mut t = ToonBuilder::new();
                t.section("race_lost", &["note"], vec![vec!["try_again".to_string()]]);
                print!("{}", t.finish());
            } else {
                if released > 0 {
                    println!("⚠ Released {released} stale in-progress task(s)");
                }
                println!("Race lost while claiming next task, please retry.");
            }
            Ok(())
        }
        ClaimResult::Claimed(task) => {
            let goal = task
                .goal_id
                .as_ref()
                .and_then(|gid| get_goal(conn, gid).ok().flatten());
            let decisions = query_decisions(conn, 6)?;
            let direction = query_direction(conn, Some(now_ts() - 7 * 24 * 3600), 6)?;
            let memories = if let Some(gid) = &task.goal_id {
                query_memories(conn, Some(gid), 10)?
            } else {
                query_memories(conn, None, 10)?
            };
            let last_failure: Option<String> = if let Some(gid) = &task.goal_id {
                conn.query_row(
                    "SELECT value FROM memories WHERE type='failure' AND (goal_id=?1 OR task_id IN (SELECT id FROM tasks WHERE goal_id=?1)) ORDER BY created_at DESC LIMIT 1",
                    params![gid],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
            } else {
                conn.query_row(
                    "SELECT value FROM memories WHERE type='failure' ORDER BY created_at DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
            };

            if out.is_json() {
                let goal_json = goal.as_ref().map(goal_to_value);
                let decisions_json: Vec<Value> = decisions
                    .iter()
                    .map(|d| json!({"what": d.0, "why": d.1, "affects": d.2, "created_at": d.3}))
                    .collect();
                let direction_json: Vec<Value> = direction
                    .iter()
                    .map(|d| json!({"content": d.0, "author": d.1, "created_at": d.2}))
                    .collect();
                let memories_json: Vec<Value> = memories.iter().map(memory_to_value).collect();
                println!(
                    "{}",
                    json!({
                        "ok": true,
                        "released_stale": released,
                        "task": {
                            "id": task.id,
                            "title": task.title,
                            "why": task.why_,
                            "description": task.description,
                            "context": task.context
                        },
                        "goal": goal_json,
                        "decisions": decisions_json,
                        "direction": direction_json,
                        "last_failure": last_failure,
                        "memories": memories_json
                    })
                );
                return Ok(());
            }

            if out.is_toon() {
                let mut t = ToonBuilder::new();
                t.section(
                    "task",
                    &["id", "title", "why"],
                    vec![vec![task.id.clone(), task.title.clone(), task.why_.clone()]],
                );
                t.section("desc", &["text"], vec![vec![task.description.clone()]]);
                if !task.context.is_empty() {
                    t.section("context", &["text"], vec![vec![task.context.clone()]]);
                }
                if let Some(g) = goal {
                    t.section(
                        "goal",
                        &["name", "why", "for_who", "success"],
                        vec![vec![g.name, g.why_, g.for_who, g.success_signal]],
                    );
                }
                t.section(
                    "decisions",
                    &["what", "why", "affects"],
                    decisions
                        .iter()
                        .map(|d| vec![d.0.clone(), d.1.clone(), d.2.clone()])
                        .collect(),
                );
                t.section(
                    "direction",
                    &["content", "author"],
                    direction
                        .iter()
                        .map(|d| vec![d.0.clone(), d.1.clone()])
                        .collect(),
                );
                if let Some(failure) = last_failure {
                    t.section("last_failure", &["note"], vec![vec![failure]]);
                }
                t.section(
                    "memories",
                    &["key", "value", "type"],
                    memories
                        .iter()
                        .map(|m| vec![m.key.clone(), m.value.clone(), m.typ.clone()])
                        .collect(),
                );
                print!("{}", t.finish());
                return Ok(());
            }

            if released > 0 {
                println!("⚠ Released {released} stale in-progress task(s)");
            }
            println!("Claimed: {}  {}", task.id, task.title);
            if !task.why_.is_empty() {
                println!("why: {}", task.why_);
            }
            if !task.description.is_empty() {
                println!("\n{}", task.description);
            }
            if !task.context.is_empty() {
                println!("\ncontext: {}", task.context);
            }
            if let Some(g) = goal {
                println!("\nGoal: {}", g.name);
                if !g.why_.is_empty() {
                    println!("  why: {}", g.why_);
                }
                if !g.for_who.is_empty() {
                    println!("  for who: {}", g.for_who);
                }
                if !g.success_signal.is_empty() {
                    println!("  success: {}", g.success_signal);
                }
            }
            if let Some(failure) = last_failure {
                println!("\nLast failure: {}", failure);
            }
            Ok(())
        }
    }
}

fn cmd_start(conn: &Connection, out: OutputCtx, agent: Option<String>, task_id: String) -> Result<(), String> {
    let agent_id = current_agent(agent.as_deref());
    let task = resolve_task(conn, &task_id)?;
    let now = now_ts();
    conn.execute(
        "UPDATE tasks SET status='in_progress', agent_id=?1, updated_at=?2 WHERE id=?3",
        params![agent_id, now, task.id],
    )
    .map_err(|e| e.to_string())?;
    if let Some(goal_id) = task.goal_id {
        sync_goal(conn, &goal_id)?;
    }

    emit_simple_ok(out, &format!("Started task {}", task.id))?;
    Ok(())
}

fn cmd_complete(
    conn: &Connection,
    out: OutputCtx,
    agent: Option<String>,
    task_id: String,
    summary: String,
) -> Result<(), String> {
    let agent_id = current_agent(agent.as_deref());
    let task = resolve_task(conn, &task_id)?;
    let now = now_ts();
    let summary_text = if summary.trim().is_empty() {
        "completed".to_string()
    } else {
        summary
    };

    conn.execute(
        "UPDATE tasks SET status='done', summary=?1, agent_id=?2, updated_at=?3, completed_at=?3 WHERE id=?4",
        params![summary_text, agent_id, now, task.id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO memories (id, goal_id, task_id, key, value, type, reasoning, source, created_at)
         VALUES (?1, ?2, ?3, 'completion_summary', ?4, 'completion', ?4, ?5, ?6)",
        params![
            gen_id(),
            task.goal_id,
            task.id,
            summary_text,
            agent_id,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    if let Some(goal_id) = task.goal_id {
        sync_goal(conn, &goal_id)?;
    }

    emit_simple_ok(out, "Task completed")?;
    Ok(())
}

fn cmd_fail(
    conn: &Connection,
    out: OutputCtx,
    agent: Option<String>,
    task_id: String,
    reason: String,
) -> Result<(), String> {
    if reason.trim().is_empty() {
        return Err("reason is required".to_string());
    }

    let task = resolve_task(conn, &task_id)?;
    let agent_id = current_agent(agent.as_deref());
    let now = now_ts();

    conn.execute(
        "UPDATE tasks SET status='todo', agent_id=NULL, updated_at=?1 WHERE id=?2",
        params![now, task.id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO memories (id, goal_id, task_id, key, value, type, reasoning, source, created_at)
         VALUES (?1, ?2, ?3, 'failure_reason', ?4, 'failure', ?4, ?5, ?6)",
        params![gen_id(), task.goal_id, task.id, reason, agent_id, now],
    )
    .map_err(|e| e.to_string())?;

    if let Some(goal_id) = task.goal_id {
        sync_goal(conn, &goal_id)?;
    }

    if out.is_json() {
        println!(
            "{}",
            json!({"ok": true, "status": "todo", "id": task.id, "title": task.title})
        );
    } else if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "task",
            &["id", "title", "status"],
            vec![vec![task.id, task.title, "todo".to_string()]],
        );
        print!("{}", t.finish());
    } else {
        println!("Task reset to todo: {}", task.id);
    }

    Ok(())
}

fn cmd_ping(conn: &Connection, out: OutputCtx, task_id: String) -> Result<(), String> {
    let id = resolve_id_prefix(conn, "tasks", &task_id)?
        .ok_or_else(|| format!("task not found: {task_id}"))?;
    let now = now_ts();
    let n = conn
        .execute(
            "UPDATE tasks SET updated_at=?1 WHERE id=?2 AND status='in_progress'",
            params![now, id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("task is not in progress".to_string());
    }

    emit_simple_ok(out, "pong")?;
    Ok(())
}

fn cmd_add_goal(
    conn: &Connection,
    out: OutputCtx,
    name: String,
    desc: Option<String>,
    priority: Option<String>,
    why: Option<String>,
    for_who: Option<String>,
    success_signal: Option<String>,
    relevant_files: Vec<String>,
    context: Option<String>,
    workspace: Option<String>,
) -> Result<(), String> {
    let id = gen_id();
    let now = now_ts();
    let cwd = workspace.unwrap_or_else(|| {
        env::current_dir()
            .ok()
            .map(|x| x.display().to_string())
            .unwrap_or_default()
    });
    let rf_json = if relevant_files.is_empty() {
        "[]".to_string()
    } else {
        serde_json::to_string(&relevant_files).unwrap_or_else(|_| "[]".to_string())
    };

    conn.execute(
        "INSERT INTO goals (id, name, description, why, for_who, success_signal, status, priority, context, tags, workspace_path, relevant_files, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'todo', ?7, ?8, '[]', ?9, ?10, ?11, ?11)",
        params![
            id,
            name,
            desc.unwrap_or_else(|| "".to_string()),
            why.unwrap_or_default(),
            for_who.unwrap_or_default(),
            success_signal.unwrap_or_default(),
            priority.unwrap_or_else(|| "medium".to_string()),
            context.unwrap_or_default(),
            cwd,
            rf_json,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    if out.is_json() {
        println!("{}", json!({"ok": true, "id": id}));
    } else if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section("goal", &["id", "name"], vec![vec![id, name]]);
        print!("{}", t.finish());
    } else {
        println!("Added goal: {}", id);
    }
    Ok(())
}

fn cmd_add_task(
    conn: &Connection,
    out: OutputCtx,
    goal_prefix: String,
    title: String,
    desc: Option<String>,
    priority: Option<String>,
    why: Option<String>,
    context: Option<String>,
    relevant_files: Vec<String>,
    tools: Vec<String>,
    acceptance_criteria: Option<String>,
    workspace: Option<String>,
) -> Result<(), String> {
    let goal_id = resolve_id_prefix(conn, "goals", &goal_prefix)?
        .ok_or_else(|| format!("goal not found: {goal_prefix}"))?;
    let id = gen_id();
    let now = now_ts();
    let cwd = workspace.unwrap_or_else(|| {
        env::current_dir()
            .ok()
            .map(|x| x.display().to_string())
            .unwrap_or_default()
    });
    let rf_json = if relevant_files.is_empty() {
        "[]".to_string()
    } else {
        serde_json::to_string(&relevant_files).unwrap_or_else(|_| "[]".to_string())
    };
    let tools_json = if tools.is_empty() {
        "[]".to_string()
    } else {
        serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string())
    };

    conn.execute(
        "INSERT INTO tasks (id, title, description, why, context, linked_files, tags, time_frame, priority, status, goal_id, execution_format, workspace_path, relevant_files, tools, acceptance_criteria, created_at, updated_at, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, '[]', '[]', 'this_week', ?6, 'todo', ?7, 'json', ?8, ?9, ?10, ?11, ?12, ?12, 'user')",
        params![
            id,
            title,
            desc.unwrap_or_default(),
            why.unwrap_or_default(),
            context.unwrap_or_default(),
            priority.unwrap_or_else(|| "medium".to_string()),
            goal_id,
            cwd,
            rf_json,
            tools_json,
            acceptance_criteria,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    sync_goal(conn, &goal_id)?;

    if out.is_json() {
        println!("{}", json!({"ok": true, "id": id, "goal_id": goal_id}));
    } else if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section("task", &["id", "goal_id", "title"], vec![vec![id, goal_id, title]]);
        print!("{}", t.finish());
    } else {
        println!("Added task: {}", id);
    }

    Ok(())
}

fn cmd_memory(conn: &Connection, out: OutputCtx, action: Option<MemoryAction>) -> Result<(), String> {
    match action {
        Some(MemoryAction::Add {
            goal_id,
            key,
            value,
        }) => {
            let gid = resolve_id_prefix(conn, "goals", &goal_id)?
                .ok_or_else(|| format!("goal not found: {goal_id}"))?;
            let now = now_ts();
            conn.execute(
                "INSERT INTO memories (id, goal_id, key, value, type, source, created_at) VALUES (?1, ?2, ?3, ?4, 'learning', 'agent', ?5)",
                params![gen_id(), gid, key, value, now],
            )
            .map_err(|e| e.to_string())?;
            emit_simple_ok(out, "Memory added")
        }
        _ => {
            let memories = query_memories(conn, None, 50)?;
            if out.is_json() {
                println!(
                    "{}",
                    json!(memories
                        .iter()
                        .map(memory_to_value)
                        .collect::<Vec<_>>())
                );
            } else if out.is_toon() {
                let mut t = ToonBuilder::new();
                t.section(
                    "memories",
                    &["id", "goal_id", "task_id", "key", "value", "type"],
                    memories
                        .iter()
                        .map(|m| {
                            vec![
                                m.id.clone(),
                                m.goal_id.clone().unwrap_or_default(),
                                m.task_id.clone().unwrap_or_default(),
                                m.key.clone(),
                                m.value.clone(),
                                m.typ.clone(),
                            ]
                        })
                        .collect(),
                );
                print!("{}", t.finish());
            } else if memories.is_empty() {
                println!("No memories.");
            } else {
                for m in memories {
                    println!("[{}] {} = {}", m.typ, m.key, m.value);
                }
            }
            Ok(())
        }
    }
}

fn cmd_decide(
    conn: &Connection,
    out: OutputCtx,
    what: String,
    why: String,
    affects: Option<String>,
) -> Result<(), String> {
    let now = now_ts();
    let id = gen_id();
    conn.execute(
        "INSERT INTO decisions (id, what, why, affects, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, what, why, affects.unwrap_or_default(), now],
    )
    .map_err(|e| e.to_string())?;

    emit_simple_ok(out, "Decision recorded")
}

fn cmd_log(conn: &Connection, out: OutputCtx, note: String) -> Result<(), String> {
    if note.trim().is_empty() {
        return Err("note is required".to_string());
    }
    let now = now_ts();
    let id = gen_id();
    let author = current_agent(None);
    conn.execute(
        "INSERT INTO direction_notes (id, content, author, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, note, author, now],
    )
    .map_err(|e| e.to_string())?;
    emit_simple_ok(out, "Direction note added")
}

fn cmd_delete(conn: &Connection, out: OutputCtx, id: String) -> Result<(), String> {
    if let Some(goal_id) = resolve_id_prefix(conn, "goals", &id)? {
        conn.execute(
            "DELETE FROM memories WHERE goal_id=?1 OR task_id IN (SELECT id FROM tasks WHERE goal_id=?1)",
            params![goal_id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM tasks WHERE goal_id=?1", params![goal_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM goals WHERE id=?1", params![goal_id])
            .map_err(|e| e.to_string())?;
        emit_simple_ok(out, "Goal deleted")?;
        return Ok(());
    }

    if let Some(task_id) = resolve_id_prefix(conn, "tasks", &id)? {
        let goal_id: Option<String> = conn
            .query_row("SELECT goal_id FROM tasks WHERE id=?1", params![task_id.clone()], |r| {
                r.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())?
            .flatten();
        conn.execute("DELETE FROM memories WHERE task_id=?1", params![task_id.clone()])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM tasks WHERE id=?1", params![task_id])
            .map_err(|e| e.to_string())?;
        if let Some(gid) = goal_id {
            let _ = sync_goal(conn, &gid);
        }
        emit_simple_ok(out, "Task deleted")?;
        return Ok(());
    }

    Err("id not found in goals or tasks".to_string())
}

fn cmd_reset(conn: &Connection, out: OutputCtx, force: bool) -> Result<(), String> {
    if !force {
        if !io::stdin().is_terminal() {
            return Err("reset requires --force in non-interactive mode".to_string());
        }
        print!("This will delete goals/tasks/memories. Type 'yes' to continue: ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.trim() != "yes" {
            emit_simple_ok(out, "Reset cancelled")?;
            return Ok(());
        }
    }

    conn.execute("DELETE FROM memories", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tasks", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM goals", []).map_err(|e| e.to_string())?;

    emit_simple_ok(out, "Reset complete")
}

fn cmd_stats(conn: &Connection, out: OutputCtx) -> Result<(), String> {
    let now = now_ts();
    let week_ago = now - 7 * 24 * 3600;

    let (total, done): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN status='done' THEN 1 ELSE 0 END),0) FROM tasks",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, 0));
    let completion_rate = if total > 0 {
        (done as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let avg_cycle_seconds: Option<f64> = conn
        .query_row(
            "SELECT AVG(completed_at - created_at) FROM tasks WHERE status='done' AND completed_at IS NOT NULL AND created_at IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();

    let activity_7d: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE COALESCE(created_at,0) >= ?1",
            params![week_ago],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut top_stmt = conn
        .prepare(
            "SELECT command, COUNT(*) as c FROM events GROUP BY command ORDER BY c DESC LIMIT 5",
        )
        .map_err(|e| e.to_string())?;
    let top_commands: Vec<(String, i64)> = top_stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let wip_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE status='in_progress'", [], |r| r.get(0))
        .unwrap_or(0);
    let stale_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status='in_progress' AND COALESCE(updated_at,created_at,0) < ?1",
            params![now - 1800],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if out.is_json() {
        println!(
            "{}",
            json!({
                "completion_rate": completion_rate,
                "avg_cycle_seconds": avg_cycle_seconds,
                "activity_7d": activity_7d,
                "top_commands": top_commands,
                "health": {
                    "wip": wip_count,
                    "stale_locks": stale_count
                }
            })
        );
        return Ok(());
    }

    if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "metrics",
            &["completion_rate", "avg_cycle_seconds", "activity_7d"],
            vec![vec![
                format!("{completion_rate:.2}"),
                avg_cycle_seconds
                    .map(|x| format!("{x:.2}"))
                    .unwrap_or_default(),
                activity_7d.to_string(),
            ]],
        );
        t.section(
            "top_commands",
            &["command", "count"],
            top_commands
                .iter()
                .map(|x| vec![x.0.clone(), x.1.to_string()])
                .collect(),
        );
        t.section(
            "health",
            &["wip", "stale_locks"],
            vec![vec![wip_count.to_string(), stale_count.to_string()]],
        );
        print!("{}", t.finish());
        return Ok(());
    }

    println!("IMI Stats");
    println!("  completion rate: {:.1}% ({}/{})", completion_rate, done, total);
    if let Some(avg) = avg_cycle_seconds {
        println!("  avg cycle time: {:.1}h", avg / 3600.0);
    } else {
        println!("  avg cycle time: n/a");
    }
    println!("  activity (7d): {} event(s)", activity_7d);

    println!("\nTop commands");
    if top_commands.is_empty() {
        println!("  (none)");
    } else {
        for (cmd, c) in top_commands {
            println!("  {}  {}", c, cmd);
        }
    }

    println!("\nHealth signals");
    println!("  in progress: {}", wip_count);
    println!("  stale locks (>30m): {}", stale_count);

    Ok(())
}

fn cmd_instructions(out: OutputCtx, target: Option<String>) -> Result<(), String> {
    let tool = target.unwrap_or_else(|| "cursor".to_string()).to_lowercase();
    let snippet = match tool.as_str() {
        "cursor" => instructions_cursor(),
        "copilot" => instructions_copilot(),
        "windsurf" => instructions_windsurf(),
        _ => return Err("tool must be one of: cursor, copilot, windsurf".to_string()),
    };

    if out.is_json() {
        println!("{}", json!({"tool": tool, "instructions": snippet}));
    } else if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section(
            "instructions",
            &["tool", "text"],
            vec![vec![tool, snippet.to_string()]],
        );
        print!("{}", t.finish());
    } else {
        println!("{}", snippet);
    }
    Ok(())
}

fn emit_simple_ok(out: OutputCtx, message: &str) -> Result<(), String> {
    if out.is_json() {
        println!("{}", json!({"ok": true, "message": message}));
    } else if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section("ok", &["message"], vec![vec![message.to_string()]]);
        print!("{}", t.finish());
    } else {
        println!("{message}");
    }
    Ok(())
}

fn emit_error(out: OutputCtx, msg: &str) {
    if out.is_json() {
        println!("{}", json!({"ok": false, "error": msg}));
    } else if out.is_toon() {
        let mut t = ToonBuilder::new();
        t.section("error", &["message"], vec![vec![msg.to_string()]]);
        print!("{}", t.finish());
    } else {
        eprintln!("{}", paint(out, "31", &format!("Error: {msg}")));
    }
}

fn discover_db_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("IMI_DB") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    if let Ok(mut dir) = env::current_dir() {
        loop {
            let candidate = dir.join(".imi").join("state.db");
            if candidate.exists() {
                return Some(candidate);
            }
            if !dir.pop() {
                break;
            }
        }
    }

    let home = env::var("HOME").ok()?;
    if cfg!(target_os = "macos") {
        Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Agents Dev")
                .join("data")
                .join("agents.db"),
        )
    } else {
        Some(PathBuf::from(home).join(".local").join("share").join("imi").join("state.db"))
    }
}

fn open_connection(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    Ok(conn)
}

fn run_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS goals (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL,
  why TEXT, for_who TEXT, success_signal TEXT, out_of_scope TEXT,
  workspace_id TEXT, status TEXT NOT NULL DEFAULT 'todo',
  priority TEXT NOT NULL DEFAULT 'medium', context TEXT, tags TEXT DEFAULT '[]',
  workspace_path TEXT, relevant_files TEXT DEFAULT '[]',
  created_at INTEGER, updated_at INTEGER, completed_at INTEGER
);
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL,
  why TEXT, context TEXT, linked_files TEXT DEFAULT '[]',
  project_id TEXT, workspace_id TEXT, assignee_type TEXT NOT NULL DEFAULT 'ai',
  agent_id TEXT, team_id TEXT, tags TEXT DEFAULT '[]',
  time_frame TEXT NOT NULL DEFAULT 'this_week', due_date INTEGER,
  priority TEXT NOT NULL DEFAULT 'medium', status TEXT NOT NULL DEFAULT 'todo',
  chat_id TEXT, summary TEXT, goal_id TEXT REFERENCES goals(id) ON DELETE SET NULL,
  plan_id TEXT, execution_format TEXT DEFAULT 'json', execution_payload TEXT,
  workspace_path TEXT, relevant_files TEXT DEFAULT '[]', tools TEXT DEFAULT '[]',
  acceptance_criteria TEXT, created_at INTEGER, updated_at INTEGER,
  completed_at INTEGER, created_by TEXT NOT NULL DEFAULT 'user'
);
CREATE TABLE IF NOT EXISTS memories (
  id TEXT PRIMARY KEY, goal_id TEXT REFERENCES goals(id) ON DELETE SET NULL,
  task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
  key TEXT NOT NULL, value TEXT NOT NULL,
  type TEXT NOT NULL DEFAULT 'learning', reasoning TEXT,
  source TEXT NOT NULL DEFAULT 'agent', created_at INTEGER
);
CREATE TABLE IF NOT EXISTS decisions (
  id TEXT PRIMARY KEY, what TEXT NOT NULL, why TEXT NOT NULL,
  affects TEXT, created_at INTEGER
);
CREATE TABLE IF NOT EXISTS direction_notes (
  id TEXT PRIMARY KEY, content TEXT NOT NULL, author TEXT, created_at INTEGER
);
CREATE TABLE IF NOT EXISTS workspaces (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT NOT NULL,
  git_remote TEXT, created_at INTEGER, updated_at INTEGER
);
CREATE TABLE IF NOT EXISTS events (
  id TEXT PRIMARY KEY, command TEXT NOT NULL, task_id TEXT,
  goal_id TEXT, agent_id TEXT, duration_ms INTEGER DEFAULT 0, created_at INTEGER
);
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY, value TEXT NOT NULL
);",
    )
    .map_err(|e| e.to_string())
}

fn register_workspace(conn: &Connection, cwd: &Path) -> Result<(), String> {
    let now = now_ts();
    let path = cwd.display().to_string();
    let name = cwd
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("workspace")
        .to_string();
    let git_remote = git_remote(cwd);

    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM workspaces WHERE path=?1 LIMIT 1",
            params![path.clone()],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some(id) = existing {
        conn.execute(
            "UPDATE workspaces SET name=?1, git_remote=?2, updated_at=?3 WHERE id=?4",
            params![name, git_remote, now, id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "INSERT INTO workspaces (id, name, path, git_remote, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![gen_id(), name, path, git_remote, now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn git_remote(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn get_goals(conn: &Connection) -> Result<Vec<GoalRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, COALESCE(description,''), COALESCE(why,''), COALESCE(for_who,''), COALESCE(success_signal,''), COALESCE(status,'todo'), COALESCE(priority,'medium'), COALESCE(created_at,0)
             FROM goals
             ORDER BY COALESCE(created_at,0) DESC",
        )
        .map_err(|e| e.to_string())?;
    let mapped = stmt.query_map([], |r| {
        Ok(GoalRow {
            id: r.get(0)?,
            name: r.get(1)?,
            description: r.get(2)?,
            why_: r.get(3)?,
            for_who: r.get(4)?,
            success_signal: r.get(5)?,
            status: r.get(6)?,
            priority: r.get(7)?,
            created_at: r.get(8)?,
        })
    })
    .map_err(|e| e.to_string())?;
    let rows = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn get_goal(conn: &Connection, id: &str) -> Result<Option<GoalRow>, String> {
    conn.query_row(
        "SELECT id, name, COALESCE(description,''), COALESCE(why,''), COALESCE(for_who,''), COALESCE(success_signal,''), COALESCE(status,'todo'), COALESCE(priority,'medium'), COALESCE(created_at,0)
         FROM goals WHERE id=?1",
        params![id],
        |r| {
            Ok(GoalRow {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                why_: r.get(3)?,
                for_who: r.get(4)?,
                success_signal: r.get(5)?,
                status: r.get(6)?,
                priority: r.get(7)?,
                created_at: r.get(8)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn get_tasks_for_goal(conn: &Connection, goal_id: &str) -> Result<Vec<TaskRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, COALESCE(description,''), COALESCE(why,''), goal_id, COALESCE(status,'todo'), COALESCE(priority,'medium'), agent_id, COALESCE(created_at,0)
             FROM tasks WHERE goal_id=?1
             ORDER BY COALESCE(updated_at, created_at, 0) DESC",
        )
        .map_err(|e| e.to_string())?;
    let mapped = stmt.query_map(params![goal_id], |r| {
        Ok(TaskRow {
            id: r.get(0)?,
            title: r.get(1)?,
            description: r.get(2)?,
            why_: r.get(3)?,
            goal_id: r.get(4)?,
            status: r.get(5)?,
            priority: r.get(6)?,
            agent_id: r.get(7)?,
            created_at: r.get(8)?,
        })
    })
    .map_err(|e| e.to_string())?;
    let rows = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn query_direction(
    conn: &Connection,
    since: Option<i64>,
    limit: i64,
) -> Result<Vec<(String, String, i64)>, String> {
    let sql = if since.is_some() {
        "SELECT content, COALESCE(author,''), COALESCE(created_at,0) FROM direction_notes WHERE COALESCE(created_at,0) >= ?1 ORDER BY COALESCE(created_at,0) DESC LIMIT ?2"
    } else {
        "SELECT content, COALESCE(author,''), COALESCE(created_at,0) FROM direction_notes ORDER BY COALESCE(created_at,0) DESC LIMIT ?1"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    if let Some(s) = since {
        let mapped = stmt
            .query_map(params![s, limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?;
        let rows = mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    } else {
        let mapped = stmt
            .query_map(params![limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?;
        let rows = mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }
}

fn query_decisions(conn: &Connection, limit: i64) -> Result<Vec<(String, String, String, i64)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT what, why, COALESCE(affects,''), COALESCE(created_at,0) FROM decisions ORDER BY COALESCE(created_at,0) DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let mapped = stmt
        .query_map(params![limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .map_err(|e| e.to_string())?;
    let rows = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn query_active_goals(conn: &Connection, limit: i64) -> Result<Vec<GoalRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, COALESCE(description,''), COALESCE(why,''), COALESCE(for_who,''), COALESCE(success_signal,''), COALESCE(status,'todo'), COALESCE(priority,'medium'), COALESCE(created_at,0)
             FROM goals
             WHERE status != 'done'
             ORDER BY COALESCE(created_at,0) DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let mapped = stmt.query_map(params![limit], |r| {
        Ok(GoalRow {
            id: r.get(0)?,
            name: r.get(1)?,
            description: r.get(2)?,
            why_: r.get(3)?,
            for_who: r.get(4)?,
            success_signal: r.get(5)?,
            status: r.get(6)?,
            priority: r.get(7)?,
            created_at: r.get(8)?,
        })
    })
    .map_err(|e| e.to_string())?;
    let rows = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn query_wip_tasks(conn: &Connection, limit: i64) -> Result<Vec<TaskRowWithGoal>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.title, COALESCE(t.description,''), COALESCE(t.why,''), t.goal_id, COALESCE(t.status,'todo'), COALESCE(t.priority,'medium'), t.agent_id, COALESCE(t.created_at,0), COALESCE(g.name,'')
             FROM tasks t
             LEFT JOIN goals g ON t.goal_id=g.id
             WHERE t.status='in_progress'
             ORDER BY COALESCE(t.updated_at,t.created_at,0) DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let mapped = stmt.query_map(params![limit], |r| {
        Ok(TaskRowWithGoal {
            id: r.get(0)?,
            title: r.get(1)?,
            description: r.get(2)?,
            why_: r.get(3)?,
            goal_id: r.get(4)?,
            status: r.get(5)?,
            priority: r.get(6)?,
            agent_id: r.get(7)?,
            created_at: r.get(8)?,
            goal_name: Some(r.get(9)?),
        })
    })
    .map_err(|e| e.to_string())?;
    let rows = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[derive(Debug, Clone)]
struct TaskRowWithGoal {
    id: String,
    title: String,
    description: String,
    why_: String,
    goal_id: Option<String>,
    status: String,
    priority: String,
    agent_id: Option<String>,
    created_at: i64,
    goal_name: Option<String>,
}

fn query_memories(conn: &Connection, goal_id: Option<&str>, limit: i64) -> Result<Vec<MemoryRow>, String> {
    let sql = if goal_id.is_some() {
        "SELECT id, goal_id, task_id, key, value, COALESCE(type,'learning'), COALESCE(source,'agent'), COALESCE(created_at,0)
         FROM memories
         WHERE goal_id=?1 OR task_id IN (SELECT id FROM tasks WHERE goal_id=?1)
         ORDER BY COALESCE(created_at,0) DESC LIMIT ?2"
    } else {
        "SELECT id, goal_id, task_id, key, value, COALESCE(type,'learning'), COALESCE(source,'agent'), COALESCE(created_at,0)
         FROM memories
         ORDER BY COALESCE(created_at,0) DESC LIMIT ?1"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    if let Some(gid) = goal_id {
        let mapped = stmt.query_map(params![gid, limit], |r| {
            Ok(MemoryRow {
                id: r.get(0)?,
                goal_id: r.get(1)?,
                task_id: r.get(2)?,
                key: r.get(3)?,
                value: r.get(4)?,
                typ: r.get(5)?,
                source: r.get(6)?,
                created_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
        let rows = mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    } else {
        let mapped = stmt.query_map(params![limit], |r| {
            Ok(MemoryRow {
                id: r.get(0)?,
                goal_id: r.get(1)?,
                task_id: r.get(2)?,
                key: r.get(3)?,
                value: r.get(4)?,
                typ: r.get(5)?,
                source: r.get(6)?,
                created_at: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
        let rows = mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }
}

fn resolve_task(conn: &Connection, prefix: &str) -> Result<TaskRow, String> {
    let id = resolve_id_prefix(conn, "tasks", prefix)?
        .ok_or_else(|| format!("task not found: {prefix}"))?;
    conn.query_row(
        "SELECT id, title, COALESCE(description,''), COALESCE(why,''), goal_id, COALESCE(status,'todo'), COALESCE(priority,'medium'), agent_id, COALESCE(created_at,0)
         FROM tasks WHERE id=?1",
        params![id],
        |r| {
            Ok(TaskRow {
                id: r.get(0)?,
                title: r.get(1)?,
                description: r.get(2)?,
                why_: r.get(3)?,
                goal_id: r.get(4)?,
                status: r.get(5)?,
                priority: r.get(6)?,
                agent_id: r.get(7)?,
                created_at: r.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

fn resolve_id_prefix(conn: &Connection, table: &str, prefix: &str) -> Result<Option<String>, String> {
    let sql = format!(
        "SELECT id FROM {table} WHERE id = ?1 OR id LIKE ?2 ORDER BY CASE WHEN id=?1 THEN 0 ELSE 1 END LIMIT 1"
    );
    let pattern = format!("{prefix}%");
    conn.query_row(&sql, params![prefix, pattern], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())
}

fn release_stale_locks(conn: &Connection) -> Result<usize, String> {
    let now = now_ts();
    conn.execute(
        "UPDATE tasks
         SET status='todo', agent_id=NULL, updated_at=?1
         WHERE status='in_progress' AND COALESCE(updated_at, created_at, 0) < ?2",
        params![now, now - 1800],
    )
    .map_err(|e| e.to_string())
}

fn claim_next_task(conn: &mut Connection, goal_id: Option<&str>, agent: &str) -> Result<ClaimResult, String> {
    let now = now_ts();
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;

    let candidate: Option<TaskClaim> = if let Some(goal) = goal_id {
        tx.query_row(
            "SELECT id, title, COALESCE(description,''), COALESCE(why,''), COALESCE(context,''), goal_id
             FROM tasks
             WHERE status='todo' AND goal_id=?1
             ORDER BY CASE priority
                WHEN 'critical' THEN 4
                WHEN 'high' THEN 3
                WHEN 'medium' THEN 2
                WHEN 'low' THEN 1
                ELSE 0 END DESC,
                COALESCE(updated_at,created_at,0) ASC
             LIMIT 1",
            params![goal],
            |r| {
                Ok(TaskClaim {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    description: r.get(2)?,
                    why_: r.get(3)?,
                    context: r.get(4)?,
                    goal_id: r.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
    } else {
        tx.query_row(
            "SELECT id, title, COALESCE(description,''), COALESCE(why,''), COALESCE(context,''), goal_id
             FROM tasks
             WHERE status='todo'
             ORDER BY CASE priority
                WHEN 'critical' THEN 4
                WHEN 'high' THEN 3
                WHEN 'medium' THEN 2
                WHEN 'low' THEN 1
                ELSE 0 END DESC,
                COALESCE(updated_at,created_at,0) ASC
             LIMIT 1",
            [],
            |r| {
                Ok(TaskClaim {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    description: r.get(2)?,
                    why_: r.get(3)?,
                    context: r.get(4)?,
                    goal_id: r.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
    };

    let Some(candidate) = candidate else {
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(ClaimResult::NoTasks);
    };

    let updated = tx
        .execute(
            "UPDATE tasks SET status='in_progress', agent_id=?1, updated_at=?2 WHERE id=?3 AND status='todo'",
            params![agent, now, candidate.id],
        )
        .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    if updated == 0 {
        return Ok(ClaimResult::RaceLost);
    }

    let verify: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT status, agent_id FROM tasks WHERE id=?1",
            params![candidate.id.clone()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some((status, owner)) = verify {
        if status == "in_progress" && owner.unwrap_or_default() == agent {
            if let Some(goal) = &candidate.goal_id {
                let _ = sync_goal(conn, goal);
            }
            return Ok(ClaimResult::Claimed(candidate));
        }
    }

    Ok(ClaimResult::RaceLost)
}

fn sync_goal(conn: &Connection, goal_id: &str) -> Result<(), String> {
    let now = now_ts();
    conn.execute(
        "UPDATE goals SET status = CASE
  WHEN NOT EXISTS(SELECT 1 FROM tasks WHERE goal_id=?1) THEN 'todo'
  WHEN EXISTS(SELECT 1 FROM tasks WHERE goal_id=?2 AND status='in_progress') THEN 'ongoing'
  WHEN EXISTS(SELECT 1 FROM tasks WHERE goal_id=?3 AND status='review') THEN 'review'
  WHEN NOT EXISTS(SELECT 1 FROM tasks WHERE goal_id=?4 AND status!='done') THEN 'done'
  WHEN EXISTS(SELECT 1 FROM tasks WHERE goal_id=?5 AND status='done') THEN 'ongoing'
  ELSE 'todo'
END, updated_at=?6 WHERE id=?7",
        params![goal_id, goal_id, goal_id, goal_id, goal_id, now, goal_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn log_event(
    conn: &Connection,
    command: &str,
    task_id: Option<&str>,
    goal_id: Option<&str>,
    agent_id: Option<&str>,
    duration_ms: i64,
) {
    let _ = conn.execute(
        "INSERT INTO events (id, command, task_id, goal_id, agent_id, duration_ms, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            gen_id(),
            command,
            task_id,
            goal_id,
            agent_id,
            duration_ms,
            now_ts()
        ],
    );
}

fn current_agent(explicit: Option<&str>) -> String {
    if let Ok(v) = env::var("IMI_AGENT_ID") {
        if !v.trim().is_empty() {
            return v;
        }
    }
    if let Some(v) = explicit {
        if !v.trim().is_empty() {
            return v.to_string();
        }
    }
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "agent".to_string())
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn ago(ts: i64) -> String {
    let diff = (now_ts() - ts).max(0);
    if diff < 60 {
        format!("{diff}s")
    } else if diff < 3600 {
        format!("{}m", diff / 60)
    } else if diff < 86400 {
        format!("{}h", diff / 3600)
    } else {
        format!("{}d", diff / 86400)
    }
}

fn status_icon(out: OutputCtx, status: &str) -> String {
    match status {
        "done" => paint(out, "32", "●"),
        "in_progress" | "ongoing" => paint(out, "33", "◐"),
        "review" => paint(out, "35", "◑"),
        "failed" | "cancelled" => paint(out, "31", "✕"),
        _ => paint(out, "90", "○"),
    }
}

fn priority_icon(out: OutputCtx, priority: &str) -> String {
    match priority {
        "critical" | "high" => paint(out, "31", "▲"),
        "low" => paint(out, "36", "▽"),
        _ => paint(out, "37", "■"),
    }
}

fn paint(out: OutputCtx, code: &str, text: &str) -> String {
    if out.color {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn base36(mut n: u128) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let chars: Vec<char> = "0123456789abcdefghijklmnopqrstuvwxyz".chars().collect();
    let mut out = String::new();
    while n > 0 {
        out.insert(0, chars[(n % 36) as usize]);
        n /= 36;
    }
    out
}

fn rand_u8() -> u8 {
    let mut b = [0u8; 1];
    if let Ok(mut f) = File::open("/dev/urandom") {
        if f.read_exact(&mut b).is_ok() {
            return b[0];
        }
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos & 0xff) as u8
}

fn gen_id() -> String {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let chars: Vec<char> = "0123456789abcdefghijklmnopqrstuvwxyz".chars().collect();
    let rand: String = (0..8)
        .map(|_| chars[(rand_u8() as usize) % 36])
        .collect();
    format!("{}{}", base36(ts_ms), rand)
}

fn goal_to_value(g: &GoalRow) -> Value {
    json!({
        "id": g.id,
        "name": g.name,
        "description": g.description,
        "why": g.why_,
        "for_who": g.for_who,
        "success_signal": g.success_signal,
        "status": g.status,
        "priority": g.priority,
        "created_at": g.created_at
    })
}

fn task_to_value(t: &TaskRow) -> Value {
    json!({
        "id": t.id,
        "title": t.title,
        "description": t.description,
        "why": t.why_,
        "goal_id": t.goal_id,
        "status": t.status,
        "priority": t.priority,
        "agent_id": t.agent_id,
        "created_at": t.created_at
    })
}

fn memory_to_value(m: &MemoryRow) -> Value {
    json!({
        "id": m.id,
        "goal_id": m.goal_id,
        "task_id": m.task_id,
        "key": m.key,
        "value": m.value,
        "type": m.typ,
        "source": m.source,
        "created_at": m.created_at
    })
}

fn wip_task_to_value(t: &TaskRowWithGoal) -> Value {
    json!({
        "id": t.id,
        "title": t.title,
        "description": t.description,
        "why": t.why_,
        "goal_id": t.goal_id,
        "goal_name": t.goal_name,
        "status": t.status,
        "priority": t.priority,
        "agent_id": t.agent_id,
        "created_at": t.created_at
    })
}

struct ToonBuilder {
    buf: String,
}

impl ToonBuilder {
    fn new() -> Self {
        Self { buf: String::new() }
    }

    fn section(&mut self, name: &str, fields: &[&str], rows: Vec<Vec<String>>) {
        if rows.is_empty() {
            return;
        }
        if !self.buf.is_empty() {
            self.buf.push('\n');
        }
        self.buf
            .push_str(&format!("{}[{}]{{{}}}:\n", name, rows.len(), fields.join(",")));
        for row in rows {
            let escaped: Vec<String> = row.into_iter().map(|v| escape_toon(&v)).collect();
            self.buf.push_str("  ");
            self.buf.push_str(&escaped.join(","));
            self.buf.push('\n');
        }
    }

    fn finish(self) -> String {
        self.buf
    }
}

fn escape_toon(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn instructions_cursor() -> &'static str {
    "# IMI Ops\n\nEvery session:\nimi status\nimi context\n\nWhen working:\nimi start <task_id>\nimi complete <task_id> \"summary\"\nimi memory add <goal_id> <key> \"insight\""
}

fn instructions_copilot() -> &'static str {
    "# IMI Ops for Copilot\n\nAt session start run:\nimi status\nimi context\n\nWhen you take work:\nimi start <task_id>\n\nWhen done:\nimi complete <task_id> \"summary\"\nimi memory add <goal_id> <key> \"what you learned\""
}

fn instructions_windsurf() -> &'static str {
    "# IMI Ops for Windsurf\n\nBoot:\nimi status\nimi context\n\nExecution loop:\nimi next\nimi start <task_id>\nimi complete <task_id> \"summary\"\nimi memory add <goal_id> <key> \"insight\""
}
