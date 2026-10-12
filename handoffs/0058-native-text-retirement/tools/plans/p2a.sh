# p2a: Tab to Name, type, live dead key Option-E (marked text, capture), then blur while the
# composition is active (retirement R1 with marked text).
[ "$($W $P scale)" = 1.0 ] || { echo "window is not on the 1x display"; exit 9; }
$W $P raise && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P keys " Byron" && $W hold 0.3 &&
$W $P key 14 alt && $W hold 0.4 &&
$C p2-01-name-deadkey-preedit &&
$W activate $PREV >/dev/null && $W hold 0.8
