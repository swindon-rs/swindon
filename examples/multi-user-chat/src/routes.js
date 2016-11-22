// src/routes.js
import React from 'react';
import { Router, Route } from 'react-router';

import Room from './components/Room';

const Routes = (props) => (
  <Router {...props}>
    <Route path="/" component={Room} />
    <Route path=":roomName" component={Room} />
  </Router>
);

export default Routes;
