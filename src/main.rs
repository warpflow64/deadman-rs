/*
use std::env;
use std::fs;
use std::process;

use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    group: Vec<Group>,
}

#[derive(Deserialize)]
struct Group {
    label: String,
    #[serde(default)]
    target: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    name:      String,
    address:   String,
    via:       Option<String>,
    relay:     Option<String>,
    os:        Option<String>,
    user:      Option<String>,
    key:       Option<String>,
    community: Option<String>,
    tcp_port:  Option<u16>,
}

/// label 先頭の '#' の数を重要度として返す (1 = 最重要)
fn heading_level(label: &str) -> usize {
    label.chars().take_while(|c| *c == '#').count().max(1)
}

/// 重要度に応じた ANSI 装飾を付けた文字列を返す
fn render_label(label: &str) -> String {
    let text = label.trim_start_matches('#').trim();
    match heading_level(label) {
        1 => format!("\x1b[1;4m{}\x1b[0m", text),  // bold + underline
        2 => format!("\x1b[1m{}\x1b[0m", text),     // bold
        _ => format!("\x1b[2m{}\x1b[0m", text),     // dim
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config.toml>", args[0]);
        process::exit(1);
    }

    let path = &args[1];

    let contents = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error: {}: {}", path, e);
        process::exit(1);
    });

    let config: Config = toml::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("error: failed to parse '{}': {}", path, e);
        process::exit(1);
    });

    for group in &config.group {
        println!("\n{}", render_label(&group.label));

        for target in &group.target {
            print!("  {:<20}  {:<40}", target.name, target.address);

            // Some だけを key=value で並べる
            let opts: Vec<String> = [
                target.via.as_deref().map(|v| format!("via={}", v)),
                target.relay.as_deref().map(|v| format!("relay={}", v)),
                target.os.as_deref().map(|v| format!("os={}", v)),
                target.user.as_deref().map(|v| format!("user={}", v)),
                target.key.as_deref().map(|v| format!("key={}", v)),
                target.community.as_deref().map(|v| format!("community={}", v)),
                target.tcp_port.map(|p| format!("tcp_port={}", p)),
            ]
            .into_iter()
            .flatten()
            .collect();

            if !opts.is_empty() {
                print!("  {}", opts.join("  "));
            }

            println!();
        }
    }
}
*/

// deadman (Rust版) の最小骨組み
//
// Cargo.toml に以下を追加:
//   tokio      = { version = "1", features = ["full"] }
//   ratatui    = "0.29"
//   crossterm  = "0.28"
//   ping_async = "..."   // あなたが使っているもの
//
// ※未コンパイルの骨組みです。クレートのバージョン差で細部の調整は要るかもしれません。

use std::collections::VecDeque;
use std::net::IpAddr;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use ping_async::{IcmpEchoRequestor, IcmpEchoStatus};
use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Row, Table};
use tokio::sync::mpsc;

const SCALE_MS: u128 = 10; // RTTバーグラフのスケール (deadman の -s 相当, 既定10ms)
const HISTORY_MAX: usize = 50;

// ===== ワーカー → UI に送る1件分の結果 =====
struct PingMsg {
    idx: usize,            // どのホストの結果か
    rtt: Option<Duration>, // None ならタイムアウト/失敗
}

// ===== UI側が保持するホスト状態 =====
struct Target {
    name: String,
    addr: IpAddr,
    up: bool,
    snt: u64,
    loss: u64,
    rtt: u128,                       // 現在RTT (ms)
    tot: u128,                       // RTT合計 (ms)
    history: VecDeque<Option<u128>>, // 最新が先頭 (push_front)
}

impl Target {
    fn new(name: String, addr: IpAddr) -> Self {
        Target {
            name,
            addr,
            up: false,
            snt: 0,
            loss: 0,
            rtt: 0,
            tot: 0,
            history: VecDeque::new(),
        }
    }

    fn apply(&mut self, rtt: Option<Duration>) {
        self.snt += 1;
        match rtt {
            Some(d) => {
                let ms = d.as_millis();
                self.up = true;
                self.rtt = ms;
                self.tot += ms;
                self.history.push_front(Some(ms));
            }
            None => {
                self.up = false;
                self.loss += 1;
                self.rtt = 0;
                self.history.push_front(None);
            }
        }
        while self.history.len() > HISTORY_MAX {
            self.history.pop_back();
        }
    }

    fn avg(&self) -> u128 {
        let ok = self.snt - self.loss;
        if ok == 0 { 0 } else { self.tot / ok as u128 }
    }

    fn loss_rate(&self) -> u32 {
        if self.snt == 0 {
            0
        } else {
            (self.loss * 100 / self.snt) as u32
        }
    }
}

// ===== スパークライン (▁▂▃▄▅▆▇█ / X) を1行ぶんのSpan列に =====
fn spark(history: &VecDeque<Option<u128>>) -> Line<'static> {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let spans: Vec<Span> = history
        .iter()
        .map(|s| match s {
            Some(ms) => {
                let lvl = ((ms / SCALE_MS) as usize).min(BARS.len() - 1);
                Span::styled(BARS[lvl].to_string(), Style::default().fg(Color::Green))
            }
            None => Span::styled("X".to_string(), Style::default().fg(Color::Red)),
        })
        .collect();
    Line::from(spans)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // 本来はここを設定ファイルから読む (deadman.conf 相当)
    let hosts: Vec<(String, IpAddr)> = vec![
        ("googleDNS".into(), "8.8.8.8".parse().unwrap()),
        ("quad9".into(), "9.9.9.9".parse().unwrap()),
    ];

    let mut targets: Vec<Target> = hosts
        .iter()
        .map(|(n, a)| Target::new(n.clone(), *a))
        .collect();

    // ワーカー → UI のチャネル
    let (tx, mut rx) = mpsc::unbounded_channel::<PingMsg>();

    // ホストごとに ping ループのタスクを起動
    for (idx, (_, addr)) in hosts.iter().enumerate() {
        let tx = tx.clone();
        let addr = *addr;
        tokio::spawn(async move {
            // 各タスクが自前の requestor を1つ持つ
            let pinger =
                match IcmpEchoRequestor::new(addr, None, Some(128), Some(Duration::from_secs(1))) {
                    Ok(p) => p,
                    Err(_) => return,
                };

            loop {
                let rtt = match pinger.send().await {
                    Ok(reply) if matches!(reply.status(), IcmpEchoStatus::Success) => {
                        Some(reply.round_trip_time())
                    }
                    _ => None, // TimedOut もエラーもまとめて失敗扱い
                };
                // UI が終了して受信端が閉じたら、このタスクも抜ける
                if tx.send(PingMsg { idx, rtt }).is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }
    drop(tx); // 親側の送信端は使わないので捨てる

    // ===== UIループ (ブロッキング) =====
    // #[tokio::main] は既定でマルチスレッド。ここでメインスレッドが止まっても
    // ping タスクは別ワーカースレッドで動き続ける。
    ratatui::run(move |mut terminal| {
        loop {
            // 1. 溜まっている結果をすべて状態に反映 (非ブロッキング)
            while let Ok(msg) = rx.try_recv() {
                targets[msg.idx].apply(msg.rtt);
            }

            // 2. 描画
            terminal.draw(|frame| {
                let header = Row::new(vec![
                    "HOSTNAME", "ADDRESS", "LOSS", "RTT", "AVG", "SNT", "HISTORY",
                ])
                .style(Style::default().add_modifier(Modifier::BOLD));

                let rows = targets.iter().map(|t| {
                    // down のホストは名前を太字に (元の挙動)
                    let name_style = if t.up {
                        Style::default()
                    } else {
                        Style::default().add_modifier(Modifier::BOLD)
                    };
                    Row::new(vec![
                        Cell::from(t.name.clone()).style(name_style),
                        Cell::from(t.addr.to_string()),
                        Cell::from(format!("{:3}%", t.loss_rate())),
                        Cell::from(format!("{:4}", t.rtt)),
                        Cell::from(format!("{:4}", t.avg())),
                        Cell::from(format!("{:4}", t.snt)),
                        Cell::from(spark(&t.history)),
                    ])
                });

                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(16),
                        Constraint::Length(16),
                        Constraint::Length(6),
                        Constraint::Length(6),
                        Constraint::Length(6),
                        Constraint::Length(6),
                        Constraint::Min(10), // 残り幅をスパークラインに
                    ],
                )
                .header(header);

                frame.render_widget(table, frame.area());
            })?;

            // 3. 入力をタイムアウト付きでポーリング
            //    read() で止めると結果が反映されないので poll に置き換える
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(k) = event::read()? {
                    match k.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') => {
                            // deadman の 'r': 全統計リセット
                            for t in targets.iter_mut() {
                                *t = Target::new(t.name.clone(), t.addr);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    })
}
