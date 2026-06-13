# deadman-rs

設定ファイル(TOML)駆動の ICMP/Ping 監視 TUI。upa/deadman(Python) の発想を
Rust(ratatui + tokio)へ移植したもの。グループ単位で重要度ラベルを付けて
一覧表示し、ターゲットごとに `via` で到達確認の方法を切り替えながら、
ライブで結果テーブル(損失率/RTT/平均/送信数/履歴スパークライン)を描画する。

## ビルドと実行

    cargo build --release
    ./target/release/deadman-rs deadman.toml

オプション:

    deadman-rs [-s SCALE_MS] <config.toml>
    -s SCALE_MS   スパークライン1段あたりの RTT(ms)。既定 10。

キー操作:

    q / Esc / Ctrl-C   終了
    r                  全ターゲットの統計をリセット

## 設定ファイル

`deadman.toml` を参照。`label` 先頭の `#` の数が重要度・強調に対応する。

    #   最重要   太字 + 下線
    ##  重要     太字
    ### 補助     淡色(dim)

各 `[[group.target]]` の `via` で到達確認方法を選ぶ。

| via            | 動作                                            | 必須フィールド            |
|----------------|-------------------------------------------------|---------------------------|
| (省略)/icmp/ping | ネイティブ ICMP（ping-async）                  | address                   |
| tcp            | TCP connect の可否と所要時間                     | address, tcp_port         |
| ssh            | relay へ SSH し、その先から ping                 | address, relay (+user/key/os) |
| snmp           | relay に対して snmpping                           | address, relay, community |
| netns          | `ip netns exec <relay> ping`                     | address, relay            |
| vrf            | `ip vrf exec <relay> ping`                       | address, relay            |

履歴の記号:

    ▁▂▃▄▅▆▇█  応答あり（RTT を SCALE_MS 段階で表示）
    X         応答なし（タイムアウト/到達不能）
    t         ssh の接続タイムアウト
    s         中継・コマンド自体の失敗（コマンド無し等）

## 動作上の注意

- ICMP(ping-async): Linux では非特権 ICMP ソケットを使うため
  `net.ipv4.ping_group_range` に実行ユーザの gid を含める必要がある。
  例: `sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"`。
  許可されていないと requestor の生成に失敗し、該当ホストは X が続く。
  Windows/macOS は非特権で動作する。
- IPv6 を ICMP で見たい場合は ping-async が IPv6 ICMP に対応している必要が
  ある。未対応なら該当ホストは X 表示になるので、必要に応じて tcp/ssh など
  別の via に切り替える。
- netns / vrf: Linux 専用で root 権限が要る（`ip netns/vrf exec` のため）。
- snmp: net-snmp 系の `snmpping` バイナリが PATH にあり、relay 側が応答する
  ことが前提。出力の `rtt min/avg/max/stddev = ...` から avg を取る。
- tcp: 監視対象ポートが開いている前提。接続成立までの時間を RTT として扱う。
- 各クレートのバージョン(ping-async / ratatui など)は手元の環境に合わせて
  Cargo.toml を調整してよい。ping-async は
  `new(addr, src, ttl, timeout)` / `send()` / `status()` / `round_trip_time()`
  という API を前提にしている。


License
=======

MIT


Contact
=======

upa@haeena.net
