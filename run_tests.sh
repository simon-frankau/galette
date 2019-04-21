#!/bin/sh

set -e

rm -rf test_tmp
mkdir test_tmp
cp testcases/*.pld test_tmp/
cd test_tmp
# Special pass for security bit flag test:
cp GAL16V8_combinatorial.pld security_bit.pld
echo '=== security_bit.pld' >> test.log
(../target/debug/galette -s security_bit.pld 2>&1 || true) >> test.log
# And the rest
for PLD in *.pld
do
    echo "=== $PLD" >> test.log
    (../target/debug/galette $PLD 2>&1 || true) >> test.log
    rm $PLD
done
cd ..

diff -ru baseline test_tmp
