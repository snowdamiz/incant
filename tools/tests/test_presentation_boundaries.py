import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import presentation_boundaries as audit
from tools.tests.test_presentation_host_stages import capture as v1_capture, catchup, recount


def capture(count=2):
    value=v1_capture(count)
    value.update(version=2,host_timing_contract=dict(audit.CONTRACT),limits=dict(audit.LIMITS),
        source_identity='requires external executable/source/package binding',
        per_present_registration_overhead=True,snapshot_after_gameplay=True,
        wake_tracking=dict(events=count,pulses=count,superseded_events=0,pulses_without_event=0,overflow=False))
    for i,a in enumerate(value['host']['attempts'],1):
        t=a['host_timing'];pulse=t['schedule']['pulse_ns']
        t.update(wake={'sequence':i,'pulse_ns':pulse,'events_since_previous_pulse':1,
                      'event':{'sequence':i,'cause':'resume_time_reached','arrived_ns':pulse-2,
                               'requested_resume_supplied':True,'requested_resume_ns':pulse-5}},
                 acquisitions=[{'began_ns':t['began_ns']+2,'ended_ns':t['began_ns']+6,'outcome':'success'},None])
    return value


class Boundaries(unittest.TestCase):
    def test_same_clock_decomposition_does_not_claim_timer_gpu_or_performance(self):
        result=audit.analyze(capture())
        self.assertTrue(result['complete_boundary_attribution'])
        self.assertFalse(result['performance_gate_passed'])
        self.assertFalse(result['display_cadence_measured'])
        row=result['attempts'][0]
        self.assertEqual(row['wake']['before_new_events_ns'],3)
        self.assertEqual(row['wake']['after_new_events_ns'],2)
        self.assertEqual(row['acquisition']['sum_call_ns'],4)
        self.assertEqual(row['acquisition']['remaining_reservation_ns'],6)

    def test_early_wait_cancelled_arrival_stays_signed(self):
        value=capture();event=value['host']['attempts'][0]['host_timing']['wake']['event']
        event.update(cause='wait_cancelled',arrived_ns=990)
        wake=audit.analyze(value)['attempts'][0]['wake']
        self.assertEqual(wake['before_new_events_ns'],-5)
        self.assertEqual(wake['after_new_events_ns'],10)
        self.assertEqual(wake['state'],'valid')

    def test_forced_resume_has_no_timer_lateness_sample(self):
        value=capture();t=value['host']['attempts'][0]['host_timing']
        t['schedule'].update(forced_resume=True,deadline_ns=None)
        t['wake']['event'].update(cause='init',requested_resume_supplied=False,requested_resume_ns=None)
        self.assertEqual(audit.analyze(value)['attempts'][0]['wake']['state'],'not_applicable')

    def test_no_wait_and_ambiguous_wakes_are_not_repaired(self):
        for count,event,state in [(0,None,'missing'),(2,True,'invalid')]:
            value=capture();w=value['host']['attempts'][0]['host_timing']['wake']
            w['events_since_previous_pulse']=count
            if event is None:w['event']=None
            result=audit.analyze(value)
            self.assertEqual(result['attempts'][0]['wake']['state'],state)
            self.assertFalse(result['complete_boundary_attribution'])

    def test_catchup_shares_one_wake_but_later_batch_cannot_reuse_it(self):
        value=capture();catchup(value)
        value['host']['attempts'][1]['host_timing']['wake']=copy.deepcopy(value['host']['attempts'][0]['host_timing']['wake'])
        self.assertTrue(audit.analyze(value)['complete_boundary_attribution'])
        value=capture();value['host']['attempts'][1]['host_timing']['wake']['event']['sequence']=1
        result=audit.analyze(value)
        self.assertEqual(result['attempts'][1]['wake']['reason'],'stale_or_inconsistent_batch_wake')
        self.assertFalse(result['complete_boundary_attribution'])

    def test_mismatched_pulse_deadline_and_regression_remain_invalid(self):
        for key,change in [('pulse',lambda t:t['wake'].__setitem__('pulse_ns',999)),
                           ('deadline',lambda t:t['wake']['event'].__setitem__('requested_resume_ns',994)),
                           ('clock',lambda t:t['wake']['event'].__setitem__('arrived_ns',1001))]:
            value=capture();change(value['host']['attempts'][0]['host_timing'])
            self.assertEqual(audit.analyze(value)['attempts'][0]['wake']['state'],'invalid',key)

    def test_failed_acquisition_is_retained_without_success_or_completed_route(self):
        value=capture(1);a=value['host']['attempts'][0];t=a['host_timing']
        a['status']='deferred';t.update(reserved_ns=None,completed_ns=1035,finished_ns=None)
        t['acquisitions'][0]['outcome']='timeout';value['rows']=[];recount(value)
        result=audit.analyze(value)
        self.assertEqual(result['attempts'][0]['acquisition']['calls'][0]['raw']['outcome'],'timeout')
        self.assertEqual(result['attempts'][0]['acquisition']['sum_call_ns'],4)
        self.assertFalse(result['complete_boundary_attribution'])

    def test_two_bounded_retry_calls_are_summed_without_view_or_bind_work(self):
        value=capture(1);t=value['host']['attempts'][0]['host_timing']
        t['acquisitions']=[{'began_ns':1021,'ended_ns':1023,'outcome':'outdated'},
                           {'began_ns':1024,'ended_ns':1028,'outcome':'success'}]
        result=audit.analyze(value)['attempts'][0]['acquisition']
        self.assertEqual(result['sum_call_ns'],6)
        self.assertEqual(result['remaining_reservation_ns'],4)
        self.assertEqual(result['state'],'valid')

    def test_open_call_and_slot_gap_never_invent_a_valid_duration(self):
        value=capture(1);t=value['host']['attempts'][0]['host_timing']
        t['acquisitions'][0].update(ended_ns=None,outcome=None)
        self.assertEqual(audit.analyze(value)['attempts'][0]['acquisition']['state'],'invalid')
        value=capture(1);t=value['host']['attempts'][0]['host_timing']
        t['acquisitions'].reverse()
        self.assertEqual(audit.analyze(value)['attempts'][0]['acquisition']['state'],'invalid')

    def test_overflow_is_not_complete_and_unknown_schema_or_fields_refuse(self):
        value=capture();value['wake_tracking']['overflow']=True
        self.assertFalse(audit.analyze(value)['complete_boundary_attribution'])
        for mutate in [lambda d:d.update(version=3),lambda d:d.update(extra='hidden'),
                       lambda d:d['host']['attempts'][0]['host_timing'].update(extra=0),
                       lambda d:d['host']['attempts'][0]['host_timing']['acquisitions'].append(None),
                       lambda d:d['wake_tracking'].update(events=True)]:
            value=capture();mutate(value)
            with self.assertRaises(ValueError):audit.analyze(value)
        with self.assertRaises(ValueError):audit.analyze(v1_capture())

    def test_paired_tail_uses_one_selected_subset_and_counts_catchup_once(self):
        value=capture();catchup(value)
        value['host']['attempts'][1]['host_timing']['wake']=copy.deepcopy(value['host']['attempts'][0]['host_timing']['wake'])
        result=audit.analyze(value)
        tail=result['paired_largest_5percent_wake_tail']
        self.assertEqual(tail['valid_samples'],1)
        self.assertEqual(tail['sums'],dict(pulse_lateness_ns=5,before_new_events_ns=3,after_new_events_ns=2))
        self.assertEqual(tail['majority_component'],'before_new_events_ns')
        reserve=result['paired_largest_5percent_reservation_tail']
        self.assertEqual(reserve['sums']['reserve_ns'],reserve['sums']['acquisition_call_ns']+reserve['sums']['other_reservation_ns'])

    def test_new_unknown_start_cause_cannot_pass_even_on_forced_resume(self):
        value=capture();t=value['host']['attempts'][0]['host_timing']
        t['schedule'].update(forced_resume=True,deadline_ns=None)
        t['wake']['event'].update(cause='unknown',requested_resume_supplied=False,requested_resume_ns=None)
        result=audit.analyze(value)
        self.assertEqual(result['attempts'][0]['wake']['reason'],'unsupported_wake_cause')
        self.assertFalse(result['complete_boundary_attribution'])

    def test_original_v1_analysis_still_accepts_its_exact_old_contract(self):
        original=v1_capture();before=json.dumps(original,sort_keys=True)
        result=audit.v1.analyze(original)
        self.assertTrue(result['complete_requested_host_timing'])
        self.assertEqual(before,json.dumps(original,sort_keys=True))
        value=capture();before=json.dumps(value,sort_keys=True);audit.analyze(value)
        self.assertEqual(before,json.dumps(value,sort_keys=True))


if __name__=='__main__':unittest.main()
