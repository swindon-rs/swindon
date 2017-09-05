var path = require('path');

module.exports = {
    entry: './index.js',
    output: {
        filename: 'bundle.js',
        path: path.resolve(__dirname, 'public/js')
    },
    module: {
        rules: [{
            loader: 'babel-loader',
            test: /\.js$/,
            exclude: /node_modules/,
        }, {
            loader: 'marko-loader',
            test: /\.marko$/,
        }, {
            loader: 'css-loader',
            test: /\.css$/,
        }],
    },
    resolve: {
        modules: process.env.NODE_PATH.split(':').concat('node_modules'),
        extensions: ['.js', '.marko'],
        mainFields: ['browser', 'jsnext:main', 'main'],
    },
    resolveLoader: {
        modules: process.env.NODE_PATH.split(':').concat('node_modules'),
        mainFields: ['jsnext:main', 'main'],
    },
};
