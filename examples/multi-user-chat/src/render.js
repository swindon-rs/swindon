import React from 'react';
import ReactDOM from 'react-dom';
import { browserHistory } from 'react-router';

import Routes from './routes';


export default function render() {
    ReactDOM.render(
      <Routes history={browserHistory} />,
      document.getElementById('root')
    );
}
