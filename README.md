# Batting Club

RustとMacroquadで作った3Dバッティングゲームです。打席で左ボタンを押し、ドラッグで緑の照準を動かして、タイミングを合わせて離します。Escキーで一時停止できます。

投球前は何度でも素振りできます。投手がボールを離した後、最初に左ボタンを離したスイングでその球を打ちます。

緑の照準はストライクゾーン内で動かせます。スマホではボタンとランナー表示を大きくします。ストライクゾーンはiPad・PCで最大約200論理ピクセルです。スマホでは幅を約222論理ピクセルまで広げ、縦横の余白に合わせて調整します。PCではボタンと表示をコンパクトに保ちます。照準はゾーンに対して同じ割合で表示します。ボール球にスイングすると空振りになります。

タイトル画面の「球速」でスロー・通常・ハイ・リアルを選んでから、10球チャレンジか練習を始めます。初期設定の通常ではストレートが約99〜112 km/h、リアルでは約129〜146 km/h相当で届きます。

10球チャレンジでは本塁打やフェア打球で得点を競います。練習では11球種から投球を指定できます。どちらも10球ごとにストライク7球・ボール3球をランダムな順番で投げます。投手は毎球0.25〜1.20秒のランダムな間を置いてから投球動作を始めます。タイトル画面で、予測点をリリース前から表示するか、リリース後に表示するかを選べます。初期設定の「リリース前」では、投球動作中のリリース0.5秒前に予測点が現れます。フォークは下、スライダーは横、シンカーは斜め下へ動きます。ワンバウンドする球は黄色で表示されます。

タイトル画面では投手の利き腕、上手投げ・横手投げ、マウス感度を変更できます。球場は中央球場と、外野フェンスが近い夕景球場の2つから選べます。両球場の外野スタンドには観客が入っています。

内野手4人と外野手3人が打球を追います。内野ゴロは一塁へ、外野に抜けた打球は二塁へ返球します。フライはノーバウンドで捕られるとアウトです。通常のグラブの範囲をわずかに外れる低い打球を、ごくまれに野手がダイビングキャッチします。内野ゴロは打者が一塁に着くより先に送球や一塁手のベースタッチが間に合うとアウトです。

本塁打には、照準の芯でタイミングを合わせて、フェア方向に柵越えまで飛ばす必要があります。通常のミートは位置とタイミングの許容範囲が広く、野手の間を抜いたり、内野ゴロで一塁への送球に勝ったりすればヒットになります。

打ったあとはランナーが自動で走ります。守備中に画面をタップするか左クリックすると、一時的に加速します。打球が外野に抜けると一塁を回って二塁を狙います。三塁への進塁は送球との競り合いを見て判断します。際どいときは二塁・三塁へ足からスライディングします。僅差の進塁では、タップ加速が足りず送球が先に着くとアウトになります。進塁を始めると、狙った塁でセーフかアウトが決まるまでプレイが続きます。

安打の種類に応じた演出を表示します。飛距離115 m以上と見込んだ本塁打では、カメラが打球を着地まで追い、打球に金色の輪と軌跡が付きます。打者は確信歩きをします。10球チャレンジの終了画面に打席ごとの球種・球速・結果・飛距離・得点が並びます。プレイ中も一時停止メニューの「打席一覧」で直近10打席を確認できます。

BGMは画面を最初にクリックすると流れます。投球、捕球、打球や結果にあわせて効果音が鳴ります。タイトル画面でBGMと効果音をそれぞれ切り替えられます。

「配球」は基本5種が初期設定で、直球中心・全11種にも切り替えられます。「変化量」は小・標準・大から選べます。主な変化球は標準でゾーンの約1/4〜1/3、大でも約1/2を目安に曲がります。練習で球種を指定したときは、その球種に変化量を適用します。

ボール球はゾーンから約4〜7 cm外れます。タップで加速している間は、走者マーカーが脈打ち、残像が付きます。

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

配布用DMGを開き、`Batting.app` を「アプリケーション」にドラッグします。配布対象はApple Siliconです。アプリはアドホック署名で、DMGは未公証です。

macOSでXcode Command Line Tools、Rust、Python 3を用意してDMGを作成できます。

```sh
rustup target add aarch64-apple-darwin
./build.sh macos-dmg
```

DMGは `dist/macos/` に生成されます。`./build.sh macos` は開発用の `dist/macos/Batting.app` を生成します。Linuxでアプリをクロスビルドする場合は、`SDKROOT` にmacOS SDKを指定してClangとlldを使います。

## Windows 10 / 11 (x64)

Visual Studio Build ToolsのC++デスクトップ開発とWindows SDK、Rustを用意し、PowerShellで実行します。成果物は `dist/windows/` に出力されます。ZIPを展開して `Batting.exe` を起動できます。

```powershell
rustup target add x86_64-pc-windows-msvc
.\scripts\build-windows.ps1
```

## iPhone・iPad

Apple Silicon MacにXcodeとiOS Simulatorのランタイムを入れます。iOS・iPadOS 16以降を対象にします。

```sh
rustup target add aarch64-apple-ios-sim aarch64-apple-ios
./build.sh ios-sim
xcrun simctl install booted dist/ios/sim/Batting.app
xcrun simctl launch booted com.ken7python.batting.ios
```

実機は `ios/Batting.xcodeproj` を開き、Signing & Capabilitiesで開発チームと必要に応じて固有のBundle IDを選びます。開発者モードが有効なiPhoneまたはiPadをXcodeで選び、Runします。コマンドラインでは開発チームを `BATTING_IOS_TEAM`、必要に応じてBundle IDを `BATTING_IOS_BUNDLE_ID` に指定して `./build.sh ios-device` を実行できます。署名済みアプリは `dist/ios/device/Batting.app` に出力されます。

## Android

macOSまたはx64 LinuxにJDK 17、Gradle 8.9、Android SDK、Python 3を用意し、`ANDROID_HOME` をSDKの場所に設定します。ARM64実機とx86_64エミュレータを対象にします。最低バージョンはAndroid 8.0です。

```sh
sdkmanager 'platforms;android-35' 'build-tools;35.0.0' 'ndk;30.0.16248370' 'platform-tools'
rustup target add aarch64-linux-android x86_64-linux-android
./build.sh android
adb install -r dist/android/batting-debug.apk
```

APKは検証用のデバッグ署名を使います。別のデバッグ鍵で署名された同じパッケージ名のアプリが端末にある場合は、そのアプリをアンインストールしてから入れ直してください。

## 開発とテスト

```sh
cargo run --locked
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

ゲームのルールと制作方針は [設計書](GAME_DESIGN.md) を参照してください。日本語UIの Batting Sans は [Noto Sans JP](https://github.com/google/fonts/tree/main/ofl/notosansjp) をもとに、使用する文字に絞って作成しています。[フォントライセンス](assets/OFL-NotoSansJP.txt)
