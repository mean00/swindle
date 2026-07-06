/** @brief Construct an RTT socket runner. */
socketRunnerRtt::socketRunnerRtt(lnFastEventGroup &eventGroup, uint32_t shift)
    : socketRunner(RUNNER_RTT_PORT, eventGroup, shift)
{
    Logger("RunnerRtt...\n");
    _connected = false;
    swindle_init_rtt();
}

void socketRunnerRtt::hook_connected()
{
    swindle_reinit_rtt();
    _connected = true;
}
void socketRunnerRtt::hook_disconnected()
{
    _connected = false;
}
void socketRunnerRtt::hook_poll()
{
    if (_connected) // connected to a debugger
    {
        if (cur_target) // and we are connected to a target...
        {
            if (swindle_rtt_enabled())
            {
                swindle_run_rtt();
            }
            else
            {
                swindle_purge_rtt();
            }
        }
    }
}

/** @brief Drop all inbound data (RTT is host-to-host; we don't write to target). */
void socketRunnerRtt::process_incoming_data()
{
    uint32_t lp = 0;

    while (1)
    {
        uint32_t rd = 0;
        uint8_t *data;
        if (readData(rd, &data))
        {
            if (!rd)
                return;
            releaseData();
        }
    }
}
/** @brief Global pointer to the RTT socket runner instance. */
socketRunnerRtt *runnerRtt = NULL;
