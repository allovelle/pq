enum IoRetention {}

// Overall idea:

// Inputs: store in Vec <- system collects
// Inputs: consume & discard <- system discards

// Outputs: emit & discard <- callers receive intermediary results
// Outputs: store in Vec <- callers access final buffer
