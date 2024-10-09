# 目的
mozcのパッケージ作成において、システム辞書として、有志が公開してくださっている辞書を含めることが目的です。  

現状、主にSudachiDictをシステム辞書として、組み込むことを目的とします。

sudachiフォルダ以下のrustプログラムが、SudachiDictをはじめとする辞書データを、Mozcのシステム辞書およびユーザー辞書型式へ変換するプログラムです。それ以外のスクリプトも残してありますが、sudachiフォルダのrustプログラムが現時点でメンテナンスされているものです。SudachiDict以外も、このプログラムで一応、変換できます。rustプログラムは、MITライセンスとしています。

## 変換プログラム概要
+ Mozcソースのid.defは更新されうるものなので、id.defは最新のものを用意してください。
+ id.defを読み込み、その品詞と、ユーザー辞書で用いられている品詞をマッピングさせます。  
ユーザー辞書の品詞の分類に変更がない限り有効です。
+ -Uオプションを用いると、ユーザー辞書型式で出力されます。省略するとシステム辞書に組み込むための型式で出力されます。
+ SudachiDictなどの辞書データの品詞判定が行えなかった場合、普通名詞と判定されます。  
id.defでの`名詞,一般,*,*,*,*,*`扱いになります。  
Mozcの内部的な品詞IDは変わることがありますので、その時点でのMozcのid.defを用いることが大事です。ただユーザー辞書型式での出力の場合には、品詞名がそのまま出力されますので、あまり意識することはないでしょう。  
+ -s SudachiDict型式を指定します。-u UtDict,-n Neologd型式を指定できます。  
+ UtDictは、それ自体が独自の品詞判定を行ったものを配布しています。そのデータが単純にユーザー辞書型式に変換されます。同じ時点id.defが使われている限りは、それなりに品詞判定が有効だと思います。  
+ Neologdやmecab-ipadicの型式も、多分、そのまま読み込んで、変換できます。品詞判定もそれなりにされると思います。
```
Usage: dict-to-mozc [-f <csv-file>] [-i <id-def>] [-U] [-s] [-n] [-u] [-P] [-S]

Dictionary to Mozc Dictionary Formats: a tool for processing dictionary files

Options:
  -f, --csv-file    path to the dictionary CSV file
  -i, --id-def      path to the Mozc id.def file
  -U, --user-dict   generate Mozc User Dictionary formats
  -s, --sudachi     target SudachiDict
  -n, --neologd     target NEologd dictionary
  -u, --utdict      target UT dictionary
  -P, --places      include place names (chimei)
  -S, --symbols     include symbols (kigou)
  --help            display usage information
```

## 使用例
SudachiDictのそれぞれのファイルをまとめたものをall.csvファイルとした場合の使用例です。
```sh
cd sudachi
# id.defの最新のものを取得
curl -LO https://github.com/google/mozc/raw/refs/heads/master/src/data/dictionary_oss/id.def
# rustプログラムのビルド
cargo build --release
# システム辞書型式への変換
./target/release/dict-to-mozc -s -i ./id.def -f all.csv > all-dict.txt
# ユーザー辞書型式への変換
./target/release/dict-to-mozc -s -i ./id.def -f all.csv -U > all-userdict.txt
```

## Neologdの例
https://github.com/neologd/mecab-ipadic-neologd/
```sh
curl -LO https://github.com/neologd/mecab-ipadic-neologd/raw/refs/heads/master/seed/mecab-user-dict-seed.20200910.csv.xz
xz -k -d mecab-user-dict-seed.20200910.csv.xz
# システム辞書型式への変換
./target/release/dict-to-mozc -n -i ./id.def -f mecab-user-dict-seed.20200910.csv > mecab-dict.txt
# ユーザー辞書型式への変換
./target/release/dict-to-mozc -n -i ./id.def -f mecab-user-dict-seed.20200910.csv -U > mecab-userdict.txt
```

# 過去の履歴
現状、このレポジトリは、変換するためのプログラムのメンテナンスがメインになっています。
沢山の種類の辞書を組み込むことは、現状、あまり注力していませんので、目的が異なってきた部分があります。
このプログラム自体を別レポジトリで、別途独立させるかもしれません。

これまでの経緯として、下記を残しています。
---
# fork of [mozcdict-ext](https://github.com/reasonset/mozcdict-ext)
下記サイトに、まとめてくださっています。感謝です。  
- [Merge UT Dictionaries merges multiple Mozc UT dictionaries into one and modify the costs.](https://github.com/utuhiro78/merge-ut-dictionaries)  
このレポジトリにおいては、上記で公開されているUT辞書と、 
- [sudachidict](https://github.com/WorksApplications/SudachiDict)  
を、mozcdict-extのスクリプトをベースに、それぞれをmozc用の辞書に変換するスクリプトを公開しているリポジトリです。 
parallelの使用により、変換時間の短縮を図っています。
mozcdict-extをベースにしているので、私が書いたスクリプト自体はGPLライセンスになるかと思います。  
このスクリプトによって生成される辞書ファイルについては、GPLライセンスの適用外になります。  
ですから、それぞれの辞書の元データのライセンスに基づいて、配布は可能になるかと思います。  
(GPL製の動画編集アプリで作成された動画がGPLライセンスに縛られないのと同じはずです。GPLライセンスのLinux上で動作するアプリがすべてGPLライセンスに縛られているわけではありません。)  
詳細は、元データの配布者のライセンスをご確認くださいませ。  
なおUT Dictionariesは、不適切と思われる単語を除去する処理が行われていますが、現状、こちらでは行っていません。

# スクリプトの使い方の例
```
ruby mecab-naist-jdic/mecab-naist-jdic.rb -i id.def -f mecab-naist-jdic-0.6.3b-20111013/naist-jdic.csv -e euc-jp
ruby utdict/utdict.rb -i id.def -f ut-dictionary1 ut-dictionary2 ...
ruby sudachi/sudachi.rb -i id.def -f sudachi/src/small_lex.csv -f sudachi/src/core_lex.csv -f sudachi/src/notcore_lex.csv 
```
-iオプションでmozcのid.defファイルを指定します。  
-fオプションで辞書ファイルを指定します。  
naist-jdic.csvがEUC-JPで配布されていましたので、--encoding,-eオプションもつけました。被っていた、--englishオプションは、-E、--Englishに変更しました。
入出力ともにUTF-8がデフォルトです。--symbolオプションも--Symbolまたは-Sに変更しています。

ユーザー辞書への変換
```
ruby utdict/user_dict.rb -i id.def -u user_dic_id.def -f ut-dictionary1 ut-dictionary2 ... >all.txt
ruby sudachi/user_dict.rb.rb -i id.def -u user_dic_id.def -f sudachi/src/small_lex.csv -f sudachi/src/core_lex.csv -f sudachi/src/notcore_lex.csv  >> all.txt 
split -d -l 1000000 --additional-suffix=.txt all.txt user-dict-
```
-uオプションでユーザー辞書への変換用のファイルを指定します。
ユーザー辞書は一つの辞書の上限が100万件です。上記は、splitコマンドで分割しています。

# ArchLinux向け AURパッケージ
- [mozc-with-jp-dict](https://aur.archlinux.org/pkgbase/mozc-with-jp-dict)
にて、AURパッケージを公開しました。  
現在、Apache License Version 2.0の辞書データのみ、有効にしています。  
それ以外の辞書データをシステム辞書に組み込みたい場合には、PKGBUILDのコメントアウトを取り除いて、ビルドしてください。(コメントアウト後、updpkgsumsコマンドを使うと簡単です。)   
sudachidictとneologdについては、[fork of mozcditc-ext](https://github.com/phoepsilonix/mozcdict-ext)の形で品詞の分類を行っています。  
ユーザー辞書として取り込みたい場合には、こちらにユーザー辞書形式の生成データがありますので、ご活用してください。  
[phoepsilonix/merge-ut-dictionaries: Merge UT Dictionaries merges multiple Mozc UT dictionaries into one and modify the costs.](https://github.com/phoepsilonix/merge-ut-dictionaries/)  

---
# mozcdict-ext

Convert external words into Mozc system dictionary

# 概要

本ツール群は Mozc-UT (Mozcdic-UT) を失ったことによる損失を埋めるための「緊急避難」として使うために作られた。

本ツール群はMozc外部のリソースからMozcシステム辞書を構築する。
これをMozcに組み込んでビルドすることにより、Mozcの語彙力を増加させることができる。

本ソフトウェアにそのようにして生成された辞書は *含まない* 。
また、 *Mozc本体も含まない* 。

このようなソフトウェアにするのはいくつか理由があるが、まず本ソフトウェアが、東風フォント事件におけるさざなみゴシックのような「緊急避難」であることを理解してほしい。
つまり、何年かかるかは分からないが、安定した開発が行われる、優れたかな漢字変換ソフトウェア及び辞書が誕生するまでの「つなぎ」である。

その意味で「つなぎ」として機能しやすいようにこのようなソフトウェアにした。
これは、Mozc以外のソフトウェアからもMozcシステム辞書からの変換とすることで利用しやすいようにし、かな漢字変換ソフトウェアの発展を促す意味もある。

Mozcdic-UTとの大きな違いは以下になる

* オープンなプロジェクトであり、ライセンスがGPL v3である
* ソフトウェアは辞書生成のためのツールであり、生成された辞書ではない
* Mozcdic-UTは一般名詞のみを対象とするが、Mozcdict-EXTは品詞を制限しない

# 使い方

## 生成の基本

各ディレクトリの `mkdict.zsh` または `mkdict.rb` は変換された辞書を生成し、標準出力に吐く。

この時以下の前提を満たす必要がある。

* スクリプトの実行はスクリプトがあるディレクトリをカレントディレクトリとして実行する
* 環境変数 `$MOZC_ID_FILE` にMozcの`id.def`ファイルのパスを入れておく必要がある

`id.def` ファイルはMozcの`src/data/dictionary_oss/id.def`に存在している。
このファイルは *本ソフトウェアには含まれない。*
ビルドにどのみちMozcが必要となるので、先にMozcのリポジトリを入手・更新しておくことが望ましい。

このようにして標準出力に吐かれた内容はMozcのシステム辞書として扱うことができ、システム辞書に組み込んでビルドすれば含めることができる。
おすすめは `src/data/dictionary_oss/dictionary09.txt` に追記することだ。

## 最後の整形

複数の辞書を生成した場合、複数の辞書にまたがる整形作業を加えるとより良い。

`.dev.utils/uniqword.rb` は`ARGF`から辞書を読み、品詞を含めて同一の語があれば除外してSTDOUTに出力する。
重複した語はSTDERRに吐かれる。

```bash
ruby uniqword.rb ~/dict/neologd.txt ~/dict/sudachi.txt > ~/dict/unified.txt
```

Mozcdic-UTと違い、固有名詞の生成を行うので、この作業はやったほうが良い。


## Archlinuxの場合

本プロジェクトとは別に `fcitx5-mozc-ext-neologd` というAURパッケージを用意している。

ARUからこのパッケージをインストールすることで外部辞書を含む形でMozcをビルドしてインストールすることができる。

なお、当該パッケージは本プロジェクトとは別のものである。

# 環境変数

## `$MOZC_ID_FILE`

必須。MOZCの `id.def` の所在を示す。

## `$WORDCLASS_ROUND`

厳密に一致する品詞がない場合に、よりおおまかな品詞に丸める。
`no`を指定するとこの処理を行わない。
次の辞書ツールで機能する。

* sudachi

## `$ERROR_ON_UNEXPECTED_CLASS`

品詞が不明な語がある場合にエラーを発生させる。
デフォルトでは発生させず、`yes`を指定した場合に発生させる。
次の辞書ツールで機能する。

* sudachi

# 実行オプション

## -e / --english

通常、このツールは「英語への変換」を除外する。
`-e` あるいは `--english` オプションをつけると、英語の変換結果を許容する。(Ruby)

## --english-proper

`--english`をつけておらず、`--english-proper`をつけた場合、英語は固有名詞である場合のみ許容する。

## -P / --no-proper

固有名詞を除外する。

## -w / --fullwidth-english

全角文字と半角カナへの変換を除外しない。

より正確には通常はOnigmoの正規表現 `/^[\p{Symbol}\p{In_CJK_Symbols_and_Punctuation}\p{Punctuation}\p{White_Space}\p{In_Halfwidth_and_Fullwidth_Forms}]+$/` にマッチする場合除外されるが、これによる除外を停止する。

## --fullwidth-english-proper

`--fullwidth-english`をつけていない場合に固有名詞のみ許容する。

## -s / --symbol

通常、このツールは変換時に支障をきたす「きごう」を変換する記号を除外するが、
`-s` あるいは `--symbol` オプションをつけると、強制的に生成に含める。

# オプションのデフォルト

コマンドラインオプションを使用せずにデフォルトのオプションを変更したい場合、設定ディレクトリ(`${XDG_CONFIG_HOME:-$HOME/.config}/mozcdict-ext`)の`config.yaml`によってデフォルトオプションを与えることができる。

例えば`--fullwidth-english`を常に有効にしたい場合は、次のようにする。

```yaml
fullwidth-english: true
```

# 除外

設定ディレクトリの`exclude.txt`ファイルを用いて、辞書への追加を回避したいパターンを指定することができる。

除外リストは、1行あたり1パターンで、読みパターンと原形パターンを1個以上の連続するホワイトスペースで区切ったものである。

パターンはそれぞれ`File.fnmatch`によってチェックされる。

例えば`ゃ`で始まる読みで変換されるすべての候補を除外したい場合は

```
ゃ*    *
```

とする。

# IssueとPR

何か問題があれば、Issueに書くか、Pull Requestを生成してほしい。

ただ、私は既にかなり手出ししている中で善意で本ソフトウェアを作っていることを理解してほしい。
つまり、IssueやPull Requestにまで手が回るかは分からない。
(少なくとも、なるべく対応したいとは思っている。)

# ライセンスとパッケージング

このソフトウェアはGPL v3でライセンスされている。
ソフトウェアは「自由に」コピーして使って良い。

一方、このソフトウェアに何か問題があったり、あるいは不足があったりしたとしても私は一切の責任を負うことはできない。
誰もがよく知っている通り、ABSOLUTELY NO WARRANTYである。

本ソフトウェアが提供するのはあくまでも辞書生成ツールである。
しかし、恐らくディストリビューションとして配布したいとすれば、それによってビルドされたMozcだろう。
このようにしてビルドされたMozcは本ツールのライセンスとは全く関係がない。
なぜならば、そのMozcに本ツールは含まれないからだ。
そのようなパッケージは、Mozcと、外部辞書として使われたリソースのライセンス・規約に従うことになるだろう。
そのようにして配布が可能であることもまた、本ソフトウェアおよび私は保証しない。

# 現在の進捗

* NEologd - 機能する
* Sudachi - 一部の品詞についてのみ生成される (実験的・開発中)

# 注意事項

* 本ソフトウェアによって生成される辞書のライセンス、および正当性について本ソフトウェアは一切関知しない

# 特に貢献を求めているもの

sudachiの`clsmap.yaml` (Sudachiの品詞分類からMozcの品詞分類への変換)

`utils/dev-by-cls.rb` を使うと品詞ごとの具体的なワードに分類して`.dev.reference/sudachi-cls`以下に吐く(`.gitignore`で指定されている)ので、これを参考に品詞分類を固める作業が進行中である。

# Dependency

* Ruby >= 3.0
* Zsh
* xz(1)
* curl(1)
