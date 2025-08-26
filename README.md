# Twitcher

Collect and monitor metrics about Bevy

## How It Works

- Commits on which to collect metrics are pushed as files to the [`queue` branch](https://github.com/bevyengine/twitcher/tree/queue)
  - A GitHub action will push new commits from the Bevy repository every hour
  - Additional commits can be pushed to collect metrics about history or more commits
- Dedicated hardware checks out the queue branch and runs the metrics collection process
- Results are pushed to the [`results` branch](https://github.com/bevyengine/twitcher/tree/results), and the commit finished is removed from the [`queue` branch](https://github.com/bevyengine/twitcher/tree/queue)
- A static website is built from the [`results` branch](https://github.com/bevyengine/twitcher/tree/results) and deployed on GitHub Pages

## How Can You Trigger Metrics Collection On A Commit

Open a PR on the [`queue` branch](https://github.com/bevyengine/twitcher/tree/queue) adding a file with the commit you want as a filename.

## How Can You Help

### Metrics Collection

You can open an issue suggesting a new metric to collect, or a PR to implement it.

They are tagged with the [`Metrics`](https://github.com/bevyengine/twitcher/issues?q=state%3Aopen%20label%3AMetrics) label.

### Website

Improvements about the website are tagged with the [`Website`](https://github.com/bevyengine/twitcher/issues?q=state%3Aopen%20label%3AWebsite) label.

### Collector

Improvements about the collector are tagged with the [`Collector`](https://github.com/bevyengine/twitcher/issues?q=state%3Aopen%20label%3ACollector) label.
