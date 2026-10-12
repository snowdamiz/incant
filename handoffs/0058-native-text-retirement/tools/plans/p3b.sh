# p3b: after refocus, Arabic commit, Tab to CJK (R3), CJK commit, select two graphemes and
# capture, Shift-Tab and Tab (R4, R5), then AX close with CJK focused (R6 at exit).
$W $P raise && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P unicode " نعم" && $W hold 0.4 &&
$W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P unicode "一" && $W hold 0.3 &&
$W $P key 123 shift && $W $P key 123 shift && $W hold 0.3 &&
$C p3-01-rtl-cjk-selection &&
$W $P key 48 shift && $W hold 0.3 && $W $P key 48 && $W hold 0.4 &&
$W $P close >/dev/null
