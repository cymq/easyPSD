# easyPSD

画像ファイルを指定のCMYKカラープロファイル（`.icc` / `.icm`）で色空間変換し、レンダリングインテントごとのソフトプルーフ比較や色調補正（色相・彩度・明度・コントラスト・ハイライト・シャドウ）を行いながらPSD形式で出力できるツールです。

デスクトップアプリ版（Windows / macOS / Linux）およびブラウザ上で動作するWebAssembly版に対応しています。

## 特徴

- **4種類のレンダリングインテント比較**: 知覚的・相対的・彩度・絶対的の各インテントでの変換結果をプレビュー比較できます。
- **CMYK変換前の色調・トーン調整**: 色相・彩度・明度・コントラスト・ハイライト・シャドウを調整した上でCMYK変換を行えます。
- **完全ローカル処理（Web版）**: WebAssembly版で選択した画像ファイルやICCプロファイルは、すべてブラウザ内で処理され、外部サーバーへ送信されることは一切ありません。

## ビルド方法

### 前提条件

プロジェクトルートにUI表示用フォント `GenInterfaceJP-Regular.ttf`（SIL Open Font License 1.1）を配置してください。

### デスクトップ版のビルド・実行

```bash
cargo run --release
```

### WebAssembly版のビルド・実行

1. `wasm32-unknown-unknown` ターゲットと `trunk` をインストールします：
   ```bash
   rustup target add wasm32-unknown-unknown
   cargo install trunk
   ```
2. [WASI SDK](https://github.com/WebAssembly/wasi-sdk) を展開し、[`.cargo/config.toml`](.cargo/config.toml)（および必要に応じて環境変数 `WASI_SDK_PATH`）のパスを自身の環境に合わせます。
3. 開発サーバーの起動または配布用バンドルのビルドを実行します：
   ```bash
   # ローカルサーバーで動作確認
   trunk serve --release

   # dist/ フォルダへ静的ファイルを出力
   trunk build --release
   ```

## ライセンス

本ソフトウェアは **GNU General Public License v3.0 (`GPL-3.0-only`)** の下で公開されています。全文は [LICENSE](LICENSE) をご確認ください。

また、本ソフトウェアが利用しているサードパーティ製ライブラリおよびフォントアセットのライセンス表記については [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) をご確認ください。
