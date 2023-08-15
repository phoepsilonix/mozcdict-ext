# fork of [mozcdict-ext](https://github.com/reasonset/mozcdict-ext)
# 目的
mozcのパッケージ作成において、システム辞書として、有志が公開してくださっている辞書を含めることが目的です。  

下記サイトに、まとめてくださっています。感謝です。  
- [Merge UT Dictionaries merges multiple Mozc UT dictionaries into one and modify the costs.](https://github.com/utuhiro78/merge-ut-dictionaries)  
このレポジトリにおいては、上記で公開されているUT辞書と、 
- [sudachidict](https://github.com/WorksApplications/SudachiDict)  
- [NAIST Japanese Dictionary](https://osdn.net/projects/naist-jdic/)  
を、mozcdict-extのスクリプトをベースに、それぞれをmozc用の辞書に変換するスクリプトを公開しているリポジトリです。 
parallelの使用により、変換時間の短縮を図っています。
mozcdict-extをベースにしているので、私が書いたスクリプト自体はGPLライセンスになるかと思います。  
このスクリプトによって生成される辞書ファイルについては、GPLライセンスの適用外になります。  
ですから、それぞれの辞書の元データのライセンスに基づいて、配布は可能になるかと思います。  
詳細は、元データの配布者のライセンスをご確認くださいませ。

# スクリプトの使い方の例
```
ruby mecab-naist-jdic/mecab-naist-jdic.rb -i id.def -f mecab-naist-jdic-0.6.3b-20111013/naist-jdic.csv -e euc-jp
ruby utdict/utdict.rb -i id.def -f ut-dictionary1 ut-dictionary2 ...
ruby sudachi/sudachi.rb -i id.def -f sudachi/src/core_lex.csv sudachi/src/notcore_lex.csv 
```
-iオプションでmozcのid.defファイルを指定します。  
-fオプションで辞書ファイルを指定します。  
naist-jdic.csvがEUC-JPで配布されていましたので、--encoding,-eオプションもつけました。被っていた、--englishオプションは、-E、--Englishに変更しました。
入出力ともにUTF-8がデフォルトです。

ユーザー辞書への変換
```
ruby utdict/user_dict.rb -i id.def -u user_dic_id.def -f ut-dictionary1 ut-dictionary2 ... >all.txt
ruby sudachi/user_dict.rb.rb -i id.def -u user_dic_id.def -f sudachi/src/core_lex.csv sudachi/src/notcore_lex.csv >> all.txt 
split -l 100000 --additional-suffix=.txt all.txt user-dict-
```
-uオプションでユーザー辞書への変換用のファイルを指定します。

# ArchLinux向け AURパッケージ
- [mozc-with-jp-dict](https://aur.archlinux.org/pkgbase/mozc-with-jp-dict)
にて、AURパッケージを公開しました。

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

## -E / --English

通常、このツールは「固有名詞以外の英単語への変換」を除外する。
`-E` あるいは `--English` オプションをつけると、固有名詞ではない英語の変換結果を許容する。

## -s / --symbol

通常、このツールは変換時に支障をきたす「きごう」を変換する記号を除外するが、
`-s` あるいは `--symbol` オプションをつけると、強制的に生成に含める。

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
