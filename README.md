# Strest

Strest is a command-line tool for stress testing web servers by sending a large number of HTTP requests. It provides insights into server performance by measuring average response times, and calculating the maximum requests per minute the server can handle and other relevant metrics.

## Features

- Send HTTP requests to a specified URL for a specified duration.
- Customize the HTTP method, headers, and request payload data.
- Measure the average response time of successful requests.
- Calculate the requests per minute (RPM) metric.
- Display real-time statistics and progress in the terminal.

## Prerequisites

- Make sure you have Rust and Cargo installed on your system. You can install Rust from [rustup.rs](https://rustup.rs/).

## Installation

To use Strest, follow these installation instructions:

1. Clone the repository to your local machine:

    ```bash
    git clone https://github.com/AC4G/strest.git
    ```

2. Change to the project directory:

    ```bash
    cd strest
    ```

3. Build the project:

    ```bash
    cargo build --release
    ```

4. Once the build is complete, you can find the executable binary in the `/target/release/` directory.

5. Copy the binary to a directory in your system's PATH to make it globally accessible:

    ```bash
    sudo cp ./target/release/strest /usr/local/bin/
    ```

## Usage

Strest is used via the command line. Here's a basic example of how to use it:

```bash
strest -u http://localhost:3000 -t 60 --no-charts
```

This command sends GET requests to `http://localhost:3000` for 60 seconds.

For more options and customization, use the --help flag to see the available command-line options and their descriptions.

```bash
strest --help
```

### Charts

By default charts are stored in the `./charts` directory where `strest` is executed. You can change the location of the charts directory by setting via the `charts` flag.

To disable charts use the `--no-charts` flag.

## Contributions

If you are interested in contributing to the project, we welcome your input and collaboration. To ensure a smooth and effective contribution process, please follow these guidelines:

1. Fork the project repository and create a dedicated branch for your work.

2. Implement your changes, enhancements, or fixes in the branch, making sure to follow the coding standards and best practices outlined in the project.

3. Submit a pull request with your changes. Our team will thoroughly review your contributions, provide feedback, and collaborate with you to incorporate your work into the project.

We greatly appreciate your interest in contributing to Strest. Your contributions help improve the tool for everyone who uses it. Thank you for considering joining our community of contributors.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Motivation 

Strest was born out of the need to stress test web servers and gain valuable insights into their performance.
