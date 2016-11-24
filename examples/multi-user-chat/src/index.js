// src/index.js
import React from 'react';
import cookie from 'cookie';
import ReactDOM from 'react-dom';
import { browserHistory } from 'react-router';

import Routes from './routes';

import './index.css';

ReactDOM.render(function() {
    let {swindon_muc_login} = cookie.parse(document.cookie);
    return <Routes history={browserHistory} login={swindon_muc_login} />
  },
  document.getElementById('root')
);
