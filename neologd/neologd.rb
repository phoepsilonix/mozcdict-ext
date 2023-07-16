#! /usr/bin/env ruby
# coding: UTF-8
require 'csv'
require 'nkf'
require 'optparse'
require 'parallel'

ID_DEF = {}
ALREADY = {}

$opts = { threads: 18, slice: 8000,
          filename: "src/seed/user-dict-seed.csv",
          fileencoding: 'UTF-8',
          need_convert: false,
          idfile: "../../mozc/src/data/dictionary_oss/id.def"
}

op = OptionParser.new
op.on("-E", "--English")
op.on('-tNUM', '--threads=NUM', Integer ) { |v| $opts[:threads] = v }
op.on('-sNUM', '--slice=NUM', Integer ) { |v| $opts[:slice] = v }
op.on('-fVAL', '--filename=VAL', String ) { |v| $opts[:filename] = v }
op.on('-iVAL', '--idfile=VAL', String ) { |v| $opts[:idfile] = v }
op.on('-eVAL', '--encoding=VAL', String ) { |v| 
  $opts[:fileencoding] = v
  $opts[:need_convert] = true
}
op.parse!(ARGV, into: $opts)

unless ENV["MOZC_ID_FILE"]
  MOZC_ID_FILE=$opts[:idfile]
else
  MOZC_ID_FILE=ENV["MOZC_ID_FILE"]
end

# Load Mozc ID definition.
File.open(MOZC_ID_FILE, "r") do |f|
  f.each do |line|
    id, expr = line.chomp.split(" ", 2)
    #id.defの品詞の末尾要素を取り除く
    expr = expr.split(",")
    expr.pop
    expr = expr.join(",")
    ID_DEF[expr] = id
  end
end

# 一番近いだろう品詞を求める。
# 単純な判定なので、誤る場合もあります。
def id_expr(clsexpr)
  expr=clsexpr.split(",")
  r=nil
  q=0
  ID_DEF.keys.each do |h|
    p=0
    expr.each do |x|
      next if x == "*"
      i = h.split(",")
      i.each do |y|
        case y
        when "*","自立","非自立"
          next
        end
        if x == y
          p = p + 1
        end
      end
    end
    if q < p
      q = p
      r = ID_DEF[h]
    end
  end
  ID_DEF[clsexpr] = r if not ID_DEF.include?(clsexpr)
  return r
end

# parallel
THREAD_NUM=$opts[:threads]
SLICE_NUM=$opts[:slice]

# Read CSV each line from file.
file = CSV.open($opts[:filename], "r", encoding: $opts[:fileencoding], liberal_parsing: false)
file.each_slice(SLICE_NUM) do |rows|
  # 表層形,左文脈ID,右文脈ID,コスト,品詞1,品詞2,品詞3,品詞4,品詞5,品詞6,原形,読み,発音
  # #24時間以内に240RT来なければ俺の嫁,1288,1288,3942,名詞,固有名詞,一般,*,*,*,#24時間以内に240RT来なければ俺の嫁,ニジュウヨジカンイナイニニヒャクヨンジュウアールティーコナケレバオレノヨメ,ニジュウヨジカンイナイニニヒャクヨンジュウアールティーコナケレバオレノヨメ
  results = Parallel.map(rows, in_threads: THREAD_NUM) do | row |
    if $opts[:need_convert]
      row.each do |x|
        next if x.nil?
        x.replace(NKF.nkf('-w', x))
      end
    end
    surface, lcxid, rcxid, cost, cls1, cls2, cls3, cls4, cls5, cls6, base, kana, pron = row
    yomi = NKF.nkf("--hiragana -w -W", kana).tr("ゐゑ", "いえ")

    # 読みがひらがな以外を含む場合はスキップ => 検証
    #if /[^\u3040-\u309F]/ !~ yomi
    next if /[\p{hiragana}\p{katakana}]/ !~ kana
    # 名詞以外の場合はスキップ => しない

    # 「地域」をスキップ。地名は郵便番号ファイルから生成する => 踏襲する
    next if cls3 == "地域"

    # 「名」をスキップ => しない

    clsexpr = [cls1, cls2, cls3, cls4, cls5, cls6].join(",").force_encoding('UTF-8')
    #clsexpr = [cls1, cls2, cls3, cls4, cls5, cls6].join(",")
    cost = cost.to_i

    ##### List class (develop feature) #####
    # puts clsexpr
    # next

    # コスト計算の処理はMozc-UTに倣っている
    mozc_cost = case
                when cost < 0
                  # コストがマイナスの場合は8000にする
                  8000
                when cost > 10000
                  # コストが10000を超える場合は10000にする
                  10000
                else
                  # コストを 6000 < cost < 7000 に調整する
                  6000 + (cost / 10)
                end

    # idは探索を行う。
    # 既知のNeologdのクラスを変換
    # 未知のものは無視する
    #next if not ID_DEF.has_key?(clsexpr)
    id = ID_DEF[clsexpr]
    if id.nil?
      id = id_expr(clsexpr)
      STDERR.puts row if id.nil?
    end
    #raise unless id      # DEVELOPMENT MODE
    next unless id

    # 英語への変換はオプションによる (デフォルトスキップ)
    # 固有名詞は受け入れる
    next if (!$opts[:English] && base =~ /^[a-zA-Z ]+$/ && !clsexpr.include?("固有名詞") )

    generic_expr = [yomi, id,  base].join(" ")
    if ALREADY[generic_expr]
      next
    else
      ALREADY[generic_expr] = true
      line_expr = [yomi, id, id, mozc_cost, base]
    end
  end
  results.map{ |x|
    next if x.nil?
    puts x.join("\t")
  }
end

