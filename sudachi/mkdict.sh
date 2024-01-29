#!/bin/sh

latest_date=$(curl -s 'http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/' | grep -o '<td>[0-9]*</td>' | grep -o '[0-9]*' | sort -n | tail -n 1)

#if [[ -e upstream ]] then;
#  rm -rf upstream;
#fi
mkdir -p upstream

if [[ -e csv ]]; then rm -rf csv; fi
mkdir -p csv

#print http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/20230110/core_lex.zip
#print http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/$date/core_lex.zip

[ -n upstream/small_lex.zip ] && curl -s "http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/$latest_date/small_lex.zip" -o upstream/small_lex.zip
[ -n upstream/core_lex.zip ] && curl -s "http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/$latest_date/core_lex.zip" -o upstream/core_lex.zip
[ -n upstream/notcore_lex.zip ] && curl -s "http://sudachi.s3-website-ap-northeast-1.amazonaws.com/sudachidict-raw/$latest_date/notcore_lex.zip" -o upstream/notcore_lex.zip

(
  cd upstream
  for i in *.zip
  do
    unzip -d ../csv $i
  done
) > /dev/null

echo $@
SYSTEMDIC=mozcdic-ut-sudachidict
USERDIC=user_dic-ut-sudachidict
source <(cargo +nightly -Z unstable-options rustc --print cfg|grep -E "target_(arch|vendor|os|env)")
TARGET="${target_arch}-${target_vendor}-${target_os}-${target_env}"
cargo +stable build --release --target $TARGET
PROG=$(find target -name dict-to-mozc)
echo "PROG=" $PROG

cat csv/small_lex.csv csv/core_lex.csv csv/notcore_lex.csv > all.csv

# ut dic
$PROG -i ../id.def -f ./all.csv -s > ./$SYSTEMDIC.tmp
awk -f ./dup.awk ./$SYSTEMDIC.tmp > ./$SYSTEMDIC.txt
rm ./$SYSTEMDIC.tmp

# userdic
$PROG -i ../id.def -f all.csv -s -U ../user_dic_id.def > ./$USERDIC.tmp
awk -f ./dup_u.awk ./$USERDIC.tmp > ./$USERDIC
split --numeric-suffixes=1 -l 1000000 --additional-suffix=.txt $USERDIC $USERDIC-
rm $USERDIC $USERDIC.tmp

mkdir -p ../release
[[ -e ../release/${USERDIC}.tar.xz ]] && rm ../release/${USERDIC}.tar.xz

tar cf ../release/${SYSTEMDIC}.tar ${SYSTEMDIC}.txt ../LICENSE
xz -9 -e ../release/${SYSTEMDIC}.tar
tar cf ../release/${USERDIC}.tar ${USERDIC}-*.txt ../LICENSE.user_dic
xz -9 -e ../release/${USERDIC}.tar

rm $USERDIC-*.txt $SYSTEMDIC.txt
rm -rf csv upstream
