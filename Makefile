paper:
	pdflatex paper && bibtex paper && pdflatex paper && pdflatex paper

clean:
	rm -f paper.aux & rm -f paper.log & rm -f paper.blg & rm -f paper.out & rm -f paper.bbl & rm -f paper.pdf
