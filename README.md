# FIFO Bounded Queue

Steps to configure, build, and test the project.

## Building

```bash
make build
```

## Running

To view the sample output:

```bash
make run
```

To run with your own settings:

```console
Usage: fifo_bounded_buffer [OPTIONS]

Options:
  -c <CONSUMERS>      Number of consumer threads [default: 1]
  -p <PRODUCERS>      Number of producer threads [default: 1]
  -i <ITEMS>          Total items to produce per thread [default: 10]
  -s <SIZE>           Size of the queue [default: 5]
  -d                  Introduce delay between enqueue/dequeue
  -h, --help          Print help
  -V, --version       Print version
```

## Testing

```bash
make check
```

## Clean

```bash
make clean
```

## Generate Documentation

Using the below command will generate the documentation and open it in your system's default browser.

```bash
make docs
```

Using the below command will generate the documentation under `target/doc` and will **_not_** open it automatically.

```bash
make docs-no-open
```

## Install Dependencies

In order to use git send-mail you need to run the following command:

```bash
make install-deps
```
