Contributing to nbd-rs
======================

The code is GPLv3 licensed, so no additional agreement is required, all contributions are considered a derivative of the code, thus GPLv3 applies to them as well. Still there are few requirements;

* Please use your full name.
* If your time contributing on this project is covered by your employer, use your work email, to make it obvious who owns the copyright for your contribution.
* Please signoff your commits, using a GPG or SSH key. (Not just the "Signed-off-by" message)
* In your first ever commit, please also include a message stating the following;

    I've read the Contribution guide at CONTRIBUTING.md. I will be signing off my commits,
    using the GPG/SSH key I use to sign this commit. The claims in DCO in Contribution Guidelines,
    applies to commits and their contents that I've signed and sent.

### Sending Patches

Community contributions, Issues and Pull Requests, will be accepted at our mirror hosted on GitLab.com.
We will review and manually import your changes back into master branch.

Repositories;
* Main repository is hosted by Rainlab, here; https://git.rlab.io/playground/nbd-rs
* Community mirror is hosted on gitlab.com, here; https://gitlab.com/rainlab-inc/nbd-rs
* An additional read-only mirror can be found on github.com, here; https://github.com/rainlab/nbd-rs
* TODO: Considering https://sr.ht/ mirror as well

### Acceptance

Most contributions are welcome. Please don't hesitate. Sometimes a prior discussion would be better.

What **MIGHT** get refused:
  + Huge refactoring (split into smaller patches instead, each being useful as well)
  + Pointless optimizations (don't optimize a code without solid reason)

What **WILL** get refused:
  + Different coding style that doesn't follow existing style
  + Any clear regression / degradation while trying to deliver something else

### Developer Certification of Origin (DCO)

The DCO is an attestation attached to every contribution made by every developer. In the commit message of the contribution, the developer simply adds a Signed-off-by statement and thereby agrees to the DCO, which you can find below or at <http://developercertificate.org/>.

```
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.


Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

#### DCO Sign-Off Methods

The DCO requires a sign-off message in the following format appear on each commit in the pull request.
Please also sign your commits using your GPG key as well.

```
Signed-off-by: Julia Child <juliachild@chef.io>
```

The DCO text can either be manually added to your commit body, or you can add either **-s** or **--signoff** to your usual git commit commands. If you forget to add the sign-off you can also amend a previous commit with the sign-off by running **git commit --amend -s**. If you've pushed your changes already, you'll need to force push your branch after this with **git push --force-with-lease**.
