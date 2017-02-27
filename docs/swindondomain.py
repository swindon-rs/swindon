from sphinx import addnodes
from sphinx.util import ws_re
from sphinx.directives import ObjectDescription
from sphinxcontrib.domaintools import custom_domain


def setup(app):
    app.add_domain(custom_domain(
        'SwindonConfig',
        name='swindon',
        label="Swindon Config",
        elements=dict(
            opt=dict(
                objname="Configuration Option",
                indextemplate="pair: %s; Config Option",
                domain_object_class=GenericObject,
            ),
            sect=dict(
                objname="Configuration Section",
                indextemplate="pair: %s; Config Section",
                domain_object_class=GenericObject,
            ),
            handler=dict(
                objname="Handler",
                indextemplate="pair: %s; Request Handler",
                domain_object_class=GenericObject,
            ),
        )))


class GenericObject(ObjectDescription):
    """
    A generic x-ref directive registered with Sphinx.add_object_type().
    """
    indextemplate = ''
    parse_node = None

    def handle_signature(self, sig, signode):
        if self.parse_node:
            name = self.parse_node(self.env, sig, signode)
        else:
            signode.clear()
            signode += addnodes.desc_name(sig, sig)
            # normalize whitespace like XRefRole does
            name = ws_re.sub('', sig)
        return name

    def add_target_and_index(self, name, sig, signode):
        targetname = '%s-%s' % (self.objtype, name)
        signode['ids'].append(targetname)
        self.state.document.note_explicit_target(signode)
        if self.indextemplate:
            colon = self.indextemplate.find(':')
            if colon != -1:
                indextype = self.indextemplate[:colon].strip()
                indexentry = self.indextemplate[colon + 1:].strip() % (name,)
            else:
                indextype = 'single'
                indexentry = self.indextemplate % (name,)
            self.indexnode['entries'].append((indextype, indexentry,
                                              targetname, '', None))
        # XXX: the only part changed is domain:
        self.env.domaindata['swindon']['objects'][self.objtype, name] = \
            self.env.docname, targetname
