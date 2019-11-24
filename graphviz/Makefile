
# dot -Tpng install.dot > install.png

graphs = shim.png install.png select.png

all: $(graphs)

%.png: %.dot
	dot -Tpng $< > $@
