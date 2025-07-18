# example-service

![Version: 0.1.37](https://img.shields.io/badge/Version-0.1.37-informational?style=flat-square) ![Type: application](https://img.shields.io/badge/Type-application-informational?style=flat-square) ![AppVersion: 0.1.1-rc51](https://img.shields.io/badge/AppVersion-0.1.1--rc51-informational?style=flat-square)

Helm chart to deploy the example-service rust application

## Maintainers

| Name | Email | Url |
| ---- | ------ | --- |
| Roxxas96 | <gomez.a.corneille@gmail.com> |  |

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| affinity | object | `{}` |  |
| autoscaling.enabled | bool | `false` | Enable/Disable Autoscaling |
| autoscaling.maxReplicas | int | `100` | Maximum number of replicas to maintain. |
| autoscaling.minReplicas | int | `1` | Minimum number of replicas to maintain. |
| autoscaling.targetCPUUtilizationPercentage | int | `80` | Target CPU utilization percentage to scale pods. This is a value between 0 and 100. |
| config.connectedServices | list | `[]` | Urls to connected services via gRPC |
| fullnameOverride | string | `""` |  |
| image.pullPolicy | string | `"IfNotPresent"` | This sets the pull policy for images. |
| image.repository | string | `"harbor.internal.roxxas96.net/example-app/example-service"` |  |
| image.tag | string | `""` | Overrides the image tag whose default is the chart appVersion. |
| imagePullSecrets | list | `[{"name":"harbor-credentials"}]` | This is for the secrets for pulling an image from a private repository more information can be found here: https://kubernetes.io/docs/tasks/configure-pod-container/pull-image-private-registry/ |
| ingress.annotations | object | `{}` | Annotations that will be added to the ingress resource |
| ingress.className | string | `""` | Used to select the ingress controller that will manage the ingress |
| ingress.enabled | bool | `false` | Enable/Disable Ingress |
| ingress.hosts | list | `[{"host":"chart-example.local","paths":[{"path":"/","pathType":"ImplementationSpecific"}]}]` | Hosts that will be served by the ingress |
| ingress.tls | list | `[]` | TLS configuration for the ingress |
| livenessProbe | object | `{"httpGet":{"path":"/health","port":"http"}}` | This is to set up the liveness and readiness probes more information can be found here: https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/ |
| logs.backtrace | bool | `true` | Decide whether backtrace is displayed when failing |
| logs.endpoint | string | `""` | Endpoint that logs are sent to |
| logs.level | string | `"info"` | Log level of the application |
| metrics.endpoint | string | `""` | Endpoint that metrics are sent to |
| metrics.pushInterval | int | `5` | Interval at which metrics are pushed to the endpoint (in seconds) |
| nameOverride | string | `""` | This is to override the chart name. |
| nodeSelector | object | `{}` |  |
| podAnnotations | object | `{}` | This is for setting Kubernetes Annotations to a Pod. For more information checkout: https://kubernetes.io/docs/concepts/overview/working-with-objects/annotations/ |
| podLabels | object | `{}` | This is for setting Kubernetes Labels to a Pod. For more information checkout: https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/ |
| podSecurityContext.fsGroup | int | `10001` |  |
| readinessProbe.httpGet.path | string | `"/ready"` |  |
| readinessProbe.httpGet.port | string | `"http"` |  |
| replicaCount | int | `1` | This will set the replicaset count more information can be found here: https://kubernetes.io/docs/concepts/workloads/controllers/replicaset/ |
| resources | object | `{}` |  |
| securityContext.capabilities.drop[0] | string | `"ALL"` |  |
| securityContext.readOnlyRootFilesystem | bool | `true` |  |
| securityContext.runAsGroup | int | `10001` |  |
| securityContext.runAsNonRoot | bool | `true` |  |
| securityContext.runAsUser | int | `10001` |  |
| service.grpcPort | int | `50051` | This sets the grpc ports more information can be found here: https://kubernetes.io/docs/concepts/services-networking/service/#field-spec-ports |
| service.httpPort | int | `3001` | This sets the http ports more information can be found here: https://kubernetes.io/docs/concepts/services-networking/service/#field-spec-ports |
| service.type | string | `"ClusterIP"` | This sets the service type more information can be found here: https://kubernetes.io/docs/concepts/services-networking/service/#publishing-services-service-types |
| serviceAccount.annotations | object | `{}` | Annotations to add to the service account |
| serviceAccount.automount | bool | `true` | Automatically mount a ServiceAccount's API credentials? |
| serviceAccount.create | bool | `true` | Specifies whether a service account should be created |
| serviceAccount.name | string | `""` | The name of the service account to use. If not set and create is true, a name is generated using the fullname template |
| tolerations | list | `[]` |  |
| traces.endpoint | string | `""` | Endpoint that traces are sent to |
| traces.sampleRatio | float | `1` | Ratio of sampled traces (0.0 - 1.0) |
| volumeMounts | list | `[]` | Additional volumeMounts on the output Deployment definition. |
| volumes | list | `[]` | Additional volumes on the output Deployment definition. |

----------------------------------------------
Autogenerated from chart metadata using [helm-docs v1.14.2](https://github.com/norwoodj/helm-docs/releases/v1.14.2)
