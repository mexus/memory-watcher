# Memory watcher

Sometimes it happens than one has to use an application that has a memory leak.
Sometimes it is possible to fix the leak. Sometimes it is not.

For the latter case you might find this tool to be useful. Simply run:
```sh
memory-watcher --name plasmashell \
    --threshold $[1024 * 1024 * 1024 * 2] \
    --check \
    --command "/usr/bin/kstart5" -- "plasmashell"
```

… and the process named `plasmashell` will be killed if its
[RSS](https://en.wikipedia.org/wiki/Resident_set_size) is more than 2 GB. Even
more than that, the process will be relaunched using the provided command
`/usr/bin/kstart5 plasmashell`. And there's even more! Before killing the
process its initial environment variables are captured (using the
`/proc/[pid]/environ` file) and the new process is launched using the vars.

It could happen that an application simply can not start right after it was
killed, so for this matter a `--check` option could be used. When specified,
the tool will check after some time *(5 seconds actually)* if the process is
running and if it is not another attempt is made. Only one, though.

## Using with systemd.timer

Here's an example of a user systemd timer (and service) that periodically runs
the tool:
```sh
$ cat .config/systemd/user/plasmashell-memory-watcher.service
[Unit]
Description=Memory watcher for plasmashell

[Service]
Type=oneshot
ExecStart=/usr/bin/memory-watcher \
          --name plasmashell \
          --threshold 2147483648 \
          --log-config /etc/memory-watcher/log4rs.yml \
          --check \
          --command "/usr/bin/kstart5" \
          -- "plasmashell"

$ cat .config/systemd/user/plasmashell-memory-watcher.timer
[Unit]
Description=Timer for the memory watcher for plasmashell every 5 minutes

[Timer]
OnBootSec=5min
OnUnitActiveSec=5min

[Install]
WantedBy=timers.target
```

To activate the timer simply run
```sh
$ systemctl --user daemon-reload
$ systemctl --user enable plasmashell-memory-watcher.timer
$ systemctl --user start plasmashell-memory-watcher.timer
```

And that's it! Have fun.

## Logs

Ah, there's one more thing. Logging is configurable, see the
[log4rs.yml](log4rs.yml) for an example and consider reading
[log4rs](https://docs.rs/log4rs/) to learn the details.
