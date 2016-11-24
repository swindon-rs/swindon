// src/routes.js
import React from 'react';
import { Router, Route, IndexRoute } from 'react-router';

import Login from './components/Login';
import Chat from './components/Login';
import Room from './components/Room';
import SelectRoom from './components/SelectRoom';

const Routes = ({login, ...props}) => (
  <Router {...props}>
    {!login && <IndexRoute component={Login} />}
    {login && <Route path="/login" component={Login} /> }
    {login &&
      <Route path="/" compoent={Chat}>
        <IndexRoute component={SelectRoom} />}
        <Route path=":roomName" component={Room} />
      </Route>}
  </Router>
);

export default Routes;
