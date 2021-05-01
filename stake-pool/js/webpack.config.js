module.exports = {
    mode: "development",
    resolve: {
        fallback: {
            "buffer": require.resolve("buffer/"),
        }
    }
}