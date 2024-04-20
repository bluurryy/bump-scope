inspect_asm::alloc_iter_u32::try_up:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB_1
	mov rax, rdx
	shr rax, 61
	jne .LBB_13
	lea r14, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov r8, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	sub r8, rax
	cmp r14, r8
	ja .LBB_4
	lea r8, [rax + r14]
	mov qword ptr [rcx], r8
	test rax, rax
	je .LBB_13
.LBB_7:
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rdx
	mov qword ptr [rsp + 24], rdi
	xor r15d, r15d
	mov rbx, rsp
	xor edx, edx
	jmp .LBB_8
.LBB_11:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r15, 4
	cmp r14, r15
	je .LBB_12
.LBB_8:
	mov ebp, dword ptr [rsi + r15]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB_11
	mov rdi, rbx
	mov r12, rsi
	call bump_scope::bump_vec::BumpVec<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB_13
	mov rsi, r12
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB_11
.LBB_1:
	mov eax, 4
	xor edx, edx
	jmp .LBB_14
.LBB_12:
	mov rax, qword ptr [rsp]
	jmp .LBB_14
.LBB_4:
	mov rbx, rdi
	mov r15, rsi
	mov rsi, rdx
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdi, rbx
	mov rdx, r12
	mov rsi, r15
	test rax, rax
	jne .LBB_7
.LBB_13:
	xor eax, eax
.LBB_14:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret