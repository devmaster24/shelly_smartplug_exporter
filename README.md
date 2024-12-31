# Shelly Smart Plug Prometheus Exporter
A prometheus exporter for the shelly smart plugs. This exporter can support one to many smart plugs at once and will
expose the rich metrics provided in prometheus format so you can scrape and visualize!

Please open an issue if you see a problem or want a new feature, I'll do my best to accommodate.

## Sample metrics
Here is a sample of the data provided by two smart plugs with this exporter.

```text
current_datetime{hostname=server} 2024-12-31T04:55:05.728Z
power_watts{hostname=server} 114.2
voltage{hostname=server} 121.5
current_amps{hostname=server} 1.018
temperature_celsius{hostname=server} 46.4
temperature_fahrenheit{hostname=server} 115.5
running_total_power_consumed_watts{hostname=server} 65115.638
current_datetime{hostname=router} 2024-12-31T04:55:05.766Z
power_watts{hostname=router} 40.1
voltage{hostname=router} 121.6
current_amps{hostname=router} 0.361
temperature_celsius{hostname=router} 52.4
temperature_fahrenheit{hostname=router} 126.4
running_total_power_consumed_watts{hostname=router} 22546.316
```

## Usage
```bash
# basic
./shelly_smartplug_exporter -i 10.0.0.2

# You can also provide multiple plugs IPs to query
./shelly_smartplug_exporter \
  --ip-addr 10.0.0.2 \
  --ip-addr 10.0.0.3
  
# With multiple IPs, it may be hard to keep track of which plug is which
# To address this, leverage the hostname-ip-mapping parameter
./shelly_smartplug_exporter \
  -i 10.0.0.2 \
  -i 10.0.0.3 \
  -m 10.0.0.2:some-plug-name \
  -m 10.0.0.3:another-plug-name

# Help
./shelly_smartplug_exporter --help

# To view the data - paste the below URL in your browser or run the CURL command
curl http://127.0.0.1:9001/metrics
```


## Advanced
You can also elect to start the web server at a custom port. Pass the `--server-port` arg to override the default
port (default = `9001`).

If you see unexpected behaviour, please check the logs of the application.


## Building
To build the application from the source yourself, you can run the below commands. Note - you must have rust installed 
on your machine.

```bash
# Update rust & cargo
rustup update

# Always wash your hands
cargo clean

# Build the executable in release mode
cargo build --release
```


## Docker
Wise choice. There is a docker image here `TODO` which you can pull from. Else, you can build your own by running the
below commands

```bash
# Ensure you are in the directory with the `Dockerfile` file present
docker build -t shelly_smartplug_exporter:latest .

# Start the container
docker run \
  --name shelly_smartplug_exporter \
  -p 9001:9001 \
  shelly_smartplug_exporter:latest \
  -i 10.0.0.2 \
  -i 10.0.0.3 \
  -m 10.0.0.2:some-plug-name \
  -m 10.0.0.3:another-plug-name
```


## Ref:
Shelly Docs: https://shelly-api-docs.shelly.cloud/gen2/ComponentsAndServices/Switch/#methods
