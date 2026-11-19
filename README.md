# Batting Club

RustとMacroquadで作った3Dバッティングゲームです。打席で左ボタンを押し、ドラッグで緑の照準を動かして、タイミングを合わせて離します。Escキーで一時停止できます。

投球前は何度でも素振りできます。投手がボールを離した後、最初に左ボタンを離したスイングでその球を打ちます。

タイトル画面の「球速」でスロー・通常・ハイ・リアルを選んでから、10球チャレンジか練習を始めます。初期設定は通常です。リアルのストレートは約141〜159 km/h相当で届きます。

10球チャレンジでは本塁打やフェア打球で得点を競います。練習では11球種の指定と通過位置の予告ができます。タイトル画面で投手の利き腕とマウス感度も変更できます。

## ビルド

macOSでRust、Bash、Python 3、Xcode Command Line Toolsを用意し、プロジェクトのルートで実行します。初回にコンパイルターゲットを追加すると、以降は `./build.sh` でWeb版とApple Silicon版をまとめて生成できます。

```sh
rustup target add wasm32-unknown-unknown aarch64-apple-darwin
./build.sh
```

## Web版

Web版だけをビルドしてローカルで遊ぶ場合は次のコマンドを使います。

```sh
rustup target add wasm32-unknown-unknown
./build.sh web
python3 -m http.server 8000 --directory dist/web
```

ブラウザで `http://localhost:8000/` を開きます。配布する際は、`dist/web/index.html` と同じディレクトリの Wasm、ローダー、ライセンスをまとめて静的HTTPサーバーへ配置します。

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
