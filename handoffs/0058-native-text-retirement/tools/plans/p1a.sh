# p1a: Tab to Name (caret capture), type, select (capture), then four field switches with one
# distinct Latin letter each, and a window blur with Name focused (retirements R1-R5).
$W $P raise && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W hold 0.3 &&
$C p1-01-name-caret &&
$W $P keys " Byron" && $W hold 0.3 &&
for i in 1 2 3 4 5; do $W $P key 123 shift || exit 7; done && $W hold 0.3 &&
$C p1-02-name-selection &&
$W $P key 124 && $W hold 0.2 &&
$W $P key 48 && $W hold 0.3 && $W $P key 119 && $W $P keys b && $W hold 0.3 &&
$W $P key 48 && $W hold 0.3 && $W $P key 119 && $W $P keys c && $W hold 0.3 &&
$W $P key 48 shift && $W hold 0.3 && $W $P key 119 && $W $P keys d && $W hold 0.3 &&
$W $P key 48 shift && $W hold 0.3 && $W $P key 119 && $W $P keys e && $W hold 0.3 &&
$W activate $PREV >/dev/null && $W hold 0.8
