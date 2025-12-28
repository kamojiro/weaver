# ADR-XXXX: タイトル（例：GitLab Pages + MkDocs をレポート正本として採用する）

- 日付: YYYY-MM-DD
- ステータス: 提案中 / 承認 / 却下 / 廃止
- レイヤ: core / infrastructure / apps / web / cross-cutting など
- 種別: ドメインモデル / データストア / ホスティング / アプリ構成 / Web/API / エージェント統合 / 運用フロー / その他
- 関連コンポーネント: 例）core.models.Report / infrastructure.git.repo_writer / apps.self_observer

---

## 1. 背景 / コンテキスト

この ADR で「何を」「なぜ」決めるのかを、プロジェクト文脈で書く。

- 目的・解決したいこと:
  - 例）レポートの正本をどこに置くかを決めたい
- 前提・制約:
  - 例）レポートは Markdown 正本で管理する
  - 例）private に運用したいが、閲覧体験はブラウザで快適にしたい
- この ADR がカバーする範囲:
  - 例）レポート閲覧のホスティング戦略のみ。検索 UI や tracker_api の詳細は別 ADR で扱う。

---

## 2. 決定

最終的に採用する方針を、できるだけ短く・具体的に。

- 例）GitLab.com の private repo + GitLab Pages + 自前 Runner を採用し、
  docs/journal/, docs/reports/ 配下の Markdown を MkDocs で静的サイト化する。

---

## 3. 選択肢と評価

検討した選択肢と、その評価を軽くまとめる。

### 採用案（本 ADR の決定）

- 概要:
  - 例）GitLab Pages + 自前 Runner + MkDocs
- メリット:
  - 例）private 運用しやすい、既存の MkDocs エコシステムを使える、PC/スマホ閲覧しやすい
- デメリット / リスク:
  - 例）自前 Runner の運用が必要、GitLab 依存が強まる

### 代替案 A

- 概要:
- 採用しなかった理由:

### 代替案 B（必要なら）

---

## 4. 根拠（評価軸と判断）

どんな観点で比較して、この決定になったかを書く。

- ビジョンとの整合:
  - 例）「Markdown 正本で管理」「既存サービスの閲覧体験を活かす」といったプロジェクト方針に合致する
- 非機能要件:
  - 例）個人利用で CI 分を節約したい → 自前 Runner で解決
  - 例）将来の検索 / RAG 用に Markdown をそのまま扱いたい
- チーム / 自分のスキル・運用コスト:
  - 例）GitLab / Docker / MkDocs に慣れているので導入コストが低い

---

## 5. 影響範囲

この決定が影響するものを列挙する。

- コード / ディレクトリ構成:
  - 例）repo 直下に mkdocs.yml, docs/ を置く
  - 例）apps からの出力先を docs/journal/YYYY/MM/DD 配下に固定する
- 既存・将来のコンポーネント:
  - 例）publisher が GitRepoWriter 経由で Markdown を出力する
  - 例）tracker_api は GitLab Pages の URL を前提に設計する
- 運用プロセス:
  - 例）レポート追加 → git commit → push → 自前 Runner でビルド → Pages 更新のフローを標準とする

---

## 6. ロールアウト / 移行方針

フェーズ付きの計画に沿って、「いつ・どう適用するか」を簡単に書く。

- フェーズ:
  - 例）フェーズ1〜2 で最初のループを完成させる
- 具体的ステップ:
  - 1. GitLab プロジェクトを作成し Pages を有効化
  - 2. ローカル PC に GitLab Runner をセットアップ
  - 3. `.gitlab-ci.yml` に MkDocs 用ジョブを追加
  - 4. 既存の docs/journal/ を push して初回ビルド
- 既存構成からの移行:
  - 例）既存のノートファイルがあれば命名規則に合わせてリネームしていく

---

## 7. オープンな論点 / フォローアップ

この ADR では決めきらなかった点や、別 ADR に切り出す前提をメモしておく。

- open question:
  - 例）検索 UI をどこまで MkDocs プラグインでやるか / 独自 web でやるか
- 別 ADR に切る予定のテーマ:
  - 例）docs ディレクトリ構造（journal / notes / themes の分離）
  - 例）activity_index / notes_index の実装方式
  - 例）エージェントフレームワーク（Pydantic AI / agno）の導入方法

---

## 8. 関連 ADR

- ADR-0003: コアドメインモデル（Report / ResearchNote / Activity）の方針
- ADR-0004: docs/ ディレクトリ構造と front matter スキーマ
- ADR-0005: レポートホスティング戦略（本 ADR）
- ADR-0006: InterestSnapshot / Task / Publication の設計方針
