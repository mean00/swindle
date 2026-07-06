/** @brief Construct a GDB socket runner. */
socketRunnerGdb::socketRunnerGdb(lnFastEventGroup &eventGroup, uint32_t shift)
    : socketRunner(RUNNER_GDB_PORT, eventGroup, shift)
{
    _connected = false;
    Logger("RunnerGdb..\n");
}

void socketRunnerGdb::hook_connected()
{
    rngdbstub_init();
    bmp_io_begin_session();
    _connected = true;
}
void socketRunnerGdb::hook_disconnected()
{
    _connected = false;
    rngdbstub_shutdown();
    bmp_io_end_session();
}
void socketRunnerGdb::hook_poll()
{
    if (_connected) // connected to a debugger
    {
        rngdbstub_poll(); // if we are un run mode, check if the target reached a breakpoint/watchpoint/...
    }
}

void socketRunnerGdb::process_incoming_data()
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
            rngdbstub_run(rd, data);
            releaseData();
            DEBUGME("\td%d\n", lp++);
        }
    }
xit:
    flushWrite();
}

/** @brief Global pointer to the GDB socket runner instance. */
socketRunnerGdb *runnerGdb = NULL;

// EOF
