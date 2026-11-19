# Batting Club

RustとMacroquadで作った3Dバッティングゲームです。打席で左ボタンを押し、ドラッグで緑の照準を動かして、タイミングを合わせて離します。Escキーで一時停止できます。

投球前は何度でも素振りできます。投手がボールを離した後、最初に左ボタンを離したスイングでその球を打ちます。

タイトル画面の「球速」でスロー・通常・ハイ・リアルを選んでから、10球チャレンジか練習を始めます。初期設定の通常ではストレートが約99〜112 km/h、リアルでは約129〜146 km/h相当で届きます。

10球チャレンジでは本塁打やフェア打球で得点を競います。練習では11球種から投球を指定できます。タイトル画面で「予告」をオンにすると、どちらのモードでもボールの通過位置を水色の点で表示します。手前で弾む球は落下地点に黄色の点が出ます。

タイトル画面では投手の利き腕、上手投げ・横手投げ、マウス感度を変更できます。球場は中央球場と、外野フェンスが近い夕景球場の2つから選べます。

内野手4人と外野手3人が打球を追います。内野ゴロは一塁へ、外野に抜けた打球は二塁へ返球します。フライはノーバウンドで捕られるとアウト。内野ゴロは打者が一塁に着くより先に送球や一塁手のベースタッチが間に合うとアウトです。

BGMは画面を最初にクリックすると流れます。投球、捕球、打球や結果にあわせて効果音が鳴ります。タイトル画面の右上でBGMと効果音をそれぞれ切り替えられます。

「配球」は直球中心・基本5種・全11種、「変化量」は小・標準・大から選べます。主な変化球は標準でゾーンの約1/4〜1/3、大でも約1/2を目安に曲がります。練習で球種を指定したときは、その球種に変化量を適用します。

## ビルド

macOSでRust、Bash、Python 3、Xcode Command Line Toolsを用意し、プロジェクトのルートで実行します。初回にコンパイルターゲットを追加すると、以降は `./build.sh` でWeb版とApple Silicon版をまとめて生成できます。

```sh
rustup target add wasm32-unknown-unknown aarch64-apple-darwin
./build.sh
```

## Web版

ブラウザでそのまま遊べます。[GitHub Pagesで Batting Club を開く](https://pages.kencode.jp/batting-rust/)

Web版だけをビルドしてローカルで遊ぶ場合は次のコマンドを使います。

```sh
rustup target add wasm32-unknown-unknown
./build.sh web
python3 -m http.server 8000 --directory dist/web
```

ブラウザで `http://localhost:8000/` を開きます。配布する際は、`dist/web/index.html` と同じディレクトリの Wasm、ローダー、ライセンスをまとめて静的HTTPサーバーへ配置します。

GitHub Pagesを更新するには `./build.sh pages` を実行し、生成されたルートの `index.html`、`.nojekyll`、`play/` をコミットして `main` にプッシュします。公開元は GitHub Pages の `main` ブランチの `/(root)` です。

## macOS版

macOSでXcode Command Line ToolsとRustを用意し、以下を実行します。

```sh
rustup target add aarch64-apple-darwin
./build.sh macos
open dist/macos/Batting.app
```

生成するアプリはApple Silicon向けです。Linuxでクロスビルドする場合は、`SDKROOT` にmacOS SDKを指定し、Clangとlldを用意して同じスクリプトを実行します。

## 開発とテスト

```sh
cargo run --locked
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

ゲームのルールと制作方針は [設計書](GAME_DESIGN.md) を参照してください。日本語UIの Batting Sans は [Noto Sans JP](https://github.com/google/fonts/tree/main/ofl/notosansjp) をもとに、使用する文字に絞って作成しています。[フォントライセンス](assets/OFL-NotoSansJP.txt)
