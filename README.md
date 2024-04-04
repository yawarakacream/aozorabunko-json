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
2. `$ cargo run <青空文庫のリポジトリへのパス> [出力先パス]`

## 対応状況

### 注記

[青空文庫 注記一覧](https://www.aozora.gr.jp/annotation/)

- レイアウト
  - [ページや段をあらためる処理](https://www.aozora.gr.jp/annotation/layout_1.html#kaicho+kaidan)
    - [x] 改丁
    - [x] 改ページ
    - [x] 改見開き
    - [x] 改段
  - [字下げ](https://www.aozora.gr.jp/annotation/layout_2.html#jisage_kakushu)
    - [x] 1 行だけの字下げ
    - [x] ブロックでの字下げ
    - [x] 凹凸の複雑な字下げ
    - [x] 地付き
    - [x] 地寄せ
  - ページの左右中央に組んである処理
    - [x] 左右中央

- [見出し](https://www.aozora.gr.jp/annotation/heading.html)
  - [x] 通常の見出し
  - [x] 同行見出し
  - [x] 窓見出し

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
    - [x] 変体仮名
      - 正体仮名にする

  - [x] [アクセント符号付きのラテン・アルファベット](https://www.aozora.gr.jp/annotation/external_character.html#accent)

- 訓点
  - [返り点](https://www.aozora.gr.jp/annotation/kunten.html#kaeriten_chuki)
    - [x] 一二点（一・二・三・四）
    - [x] 上下点（上・中・下）
    - [x] 甲乙点（甲・乙・丙・丁）
    - [x] レ点
    - [ ] 竪点
  - [x] [訓点送り仮名](https://www.aozora.gr.jp/annotation/kunten.html#kunten_okurigana_chuki)

- 強調
  - [x] [傍点](https://www.aozora.gr.jp/annotation/emphasis.html#boten_chuki)
  - [x] [傍線](https://www.aozora.gr.jp/annotation/emphasis.html#bosen_chuki)
  - [x] [太字と斜体](https://www.aozora.gr.jp/annotation/emphasis.html#futoji_gothic,shatai_italic)

- 画像とキャプション
  - [ ] [写真や図版、挿絵などの画像](https://www.aozora.gr.jp/annotation/graphics.html#gazo_chuki)
  - [ ] [キャプション](https://www.aozora.gr.jp/annotation/graphics.html#caption)

- その他
  - [訂正とママ](https://www.aozora.gr.jp/annotation/etc.html#teisei_mama)
    無視する
    - [x] `［＃「○○」に「ママ」の注記］`
    - [x] `［＃「○○」は底本では「●●」］`
    - [x] `［＃ルビの「○○」は底本では「●●」］`
    - [x] `［＃「○○」はママ］`
    - [x] `［＃ルビの「○○」はママ］`
    
  - [x] [ルビとルビのように付く文字](https://www.aozora.gr.jp/annotation/etc.html#ruby)
  - [ ] [縦組み中で横に並んだ文字](https://www.aozora.gr.jp/annotation/etc.html#tatechu_yoko)
  - [x] [割り注](https://www.aozora.gr.jp/annotation/etc.html#warichu)
  - [ ] [行右小書き、行左小書き文字（縦組み）](https://www.aozora.gr.jp/annotation/etc.html#gyomigi_gyohidari)
  - [ ] [上付き小文字、下付き小文字（横組み）](https://www.aozora.gr.jp/annotation/etc.html#uwatsuki_shitatsuki)
  - [ ] [字詰め](https://www.aozora.gr.jp/annotation/etc.html#jizume)
  - [ ] [罫囲み](https://www.aozora.gr.jp/annotation/etc.html#keigakomi)
  - [ ] [横組み](https://www.aozora.gr.jp/annotation/etc.html#yokogumi)
  - [ ] [文字サイズ](https://www.aozora.gr.jp/annotation/etc.html#moji_size)

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
