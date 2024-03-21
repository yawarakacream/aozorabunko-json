# aozorabunko-json

[青空文庫](https://www.aozora.gr.jp/) を Unicode の JSON に変換する

- `index_pages/list_person_all_extended_utf8.csv` をもとに次のデータを収集：
  - 本
  - 著者
  - 本 対 著者
- `テキストファイル URL` から作品をパースして収集
  - 収集する条件：
    - https://www.aozora.gr.jp/ から始まる
    - `作品著作権フラグ` 及び `人物著作権フラグ` がともに `"なし"`

## 実行

1. [青空文庫のリポジトリ](https://github.com/aozorabunko/aozorabunko) を clone する
2. `$ cargo run <青空文庫のリポジトリへのパス> <出力先パス>`

## 対応状況

### 注記

[青空文庫 注記一覧](https://www.aozora.gr.jp/annotation/)

- 外字  
  出力は Unicode なので可能な限り注記から文字を求めて出力する。  
  底本でのページ番号や行数は保存しない。  

  - [第 1 第 2 水準にない漢字](https://www.aozora.gr.jp/annotation/external_character.html#0208gaiji_chuki)
    - [x] JIS X 2013
    - [x] Unicode
    - [ ] JIS X 2013 にも Unicode にもない
      - 現状は `※［{}］` そのままにしている

  - [特殊な仮名や記号など](https://www.aozora.gr.jp/annotation/external_character.html#tokushu)
    - [x] JIS X 2013
    - [x] くの字点
      - 「〱（U+3031）」「〲（U+3032）」にする
    - [x] 変体仮名
      - 正体仮名にする

  - [x] [アクセント符号付きのラテン・アルファベット](https://www.aozora.gr.jp/annotation/external_character.html#accent)

- その他

  - [x] [ルビとルビのように付く文字](https://www.aozora.gr.jp/annotation/etc.html#ruby)

- 青空文庫をこえた利用から

  - [本文終わり](https://www.aozora.gr.jp/annotation/extra.html#end_annotation)
    - [x] `底本：`
    - [ ] `［＃本文終わり］`
      - 利用例が見つからないので保留

## 参考

- [青空文庫 注記一覧](https://www.aozora.gr.jp/annotation/)
- [青空文庫編 耕作員手帳](https://www.aozora.gr.jp/guide/techo.html)
- [青空文庫編 入力ファイルを「テキスト版」に仕上げるために](https://www.aozora.gr.jp/KOSAKU/textfile_checklist/index.html)
- [青空文庫編 青空文庫収録ファイルの取り扱い規準](https://www.aozora.gr.jp/guide/kijyunn.html)
- [青空文庫編 青空部湖収録ファイルへの記載事項](https://www.aozora.gr.jp/guide/kisai.html)
- [青空文庫編 青空文庫 FAQ](https://www.aozora.gr.jp/guide/aozora_bunko_faq.html)

- [青空文庫 組版案内](http://kumihan.aozora.gr.jp/)（古い？）

- [aozorabunko_text](https://github.com/aozorahack/aozorabunko_text)
