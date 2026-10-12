# p1b: three blur/refocus cycles, each with a different field focused and one letter typed
# (retirements R6-R8). Refocus does not restore UI focus, so each cycle Tabs to its field.
$W $P raise && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P keys f && $W hold 0.3 &&
$W activate $PREV >/dev/null && $W hold 0.8 &&
$W activate $P >/dev/null && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P keys g && $W hold 0.3 &&
$W activate $PREV >/dev/null && $W hold 0.8 &&
$W activate $P >/dev/null && $W hold 0.6 &&
$W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W $P key 48 && $W hold 0.4 &&
$W $P key 119 && $W $P keys h && $W hold 0.3 &&
$W activate $PREV >/dev/null && $W hold 0.8
