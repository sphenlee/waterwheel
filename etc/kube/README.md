Kubernetes Waterwheel Setup
===========================

This folder contains the files needed to get a basic Waterwheel setup 
running in Kubernetes.

This example will use `minikube`.

1. enable the `ingress` and `ingress-dns` addons:

   ```
   minikube addons enable ingress
   minikube addons enable ingress-dns
   ```

2. follow the instructions from Minikube to configure DNS resolutions
   for `*.kube` domain.

   For example on Ubuntu using systemd-resolved:

   ```bash
   IFACE="$(ip -json route get "$(minikube ip)" | jq -r .[0].dev)"
   resolvectl dns "$IFACE" "$(minikube ip)"
   resolvectl domain "$IFACE" ~kube
   ```

3. generate the required RSA keypair and TLS certificates.
   This will require `openssl` and `mkcert` installed.

   ```bash
   cd etc/kube/
   just gen-keypair
   just gen-tlscert
   ```

4. apply the kubernetes descriptors:

   ``` 
   just apply
   ```

5. check the web interface at https://waterwheel.kube/ . You will need to 
   login as `fry` + `fry`.

6. create the sample jobs

   ```bash
   cd sample/
   eval $(minikube docker-env)
   export WATERWHEEL_ADDR=https://waterwheel.kube/
   just deploy
   ```
