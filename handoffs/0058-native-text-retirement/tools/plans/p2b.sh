# p2b: after refocus, E in Arabic (no dead-key carry-over), Shift-Tab to Name (R2), dead key,
# pointer click into CJK while marked (R3; the click also submits CJK: known limitation),
# E in CJK, Shift-Tab twice back to Name (R4, R5), live dead-key commit, capture all fields,
# dead key again and AX close while marked (R6 at exit).
[ "$($W $P scale)" = 1.0 ] || { echo "window is not on the 1x display"; exit 9; }
$W $P raise && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P key 14 && $W hold 0.3 &&
$W $P key 48 shift && $W hold 0.4 &&
$W $P key 119 && $W $P key 14 alt && $W hold 0.4 &&
$W $P click 192 165 && $W hold 0.5 &&
$W $P key 119 && $W $P key 14 && $W hold 0.3 &&
$W $P key 48 shift && $W hold 0.3 && $W $P key 48 shift && $W hold 0.4 &&
$W $P key 119 && $W $P key 14 alt && $W hold 0.3 && $W $P key 14 && $W hold 0.4 &&
$C p2-02-after-composition-retirements &&
$W $P key 14 alt && $W hold 0.4 &&
$W $P close >/dev/null
