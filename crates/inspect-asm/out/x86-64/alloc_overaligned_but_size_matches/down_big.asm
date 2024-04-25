inspect_asm::alloc_overaligned_but_size_matches::down_big:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, rax
	sub rdx, qword ptr [rcx + 8]
	cmp rdx, 40
	jb .LBB_2
	and rax, -4
	add rax, -40
	mov qword ptr [rcx], rax
.LBB_3:
	mov rcx, qword ptr [rsi + 32]
	mov qword ptr [rax + 32], rcx
	movups xmm0, xmmword ptr [rsi]
	movups xmm1, xmmword ptr [rsi + 16]
	movups xmmword ptr [rax + 16], xmm1
	movups xmmword ptr [rax], xmm0
	pop rbx
	ret
.LBB_2:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	jmp .LBB_3